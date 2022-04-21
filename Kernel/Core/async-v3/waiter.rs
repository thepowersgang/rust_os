// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async-v3/waiter.rs
//! Core waiter object
//!
//! The `Waiter` object is the top level of an async context, wrapping a provided number of
//! async operations and allowing the owner to wait until an operation is complete.
//!
//! Each operation is able to utilise a fixed-size state stack (up to 256 bytes) to store
//! intermediate data for when an async operation requres mutiple sub-operations (e.g. acquiring
//! a mutex then performing hardware IO)
#[allow(unused_imports)]
use crate::prelude::*;
use crate::threads::SleepObject;
use core::sync::atomic::{self, Ordering};
use core::marker::PhantomData;
use crate::sync::Spinlock;

/// Trait representing a saved piece of async state
pub trait Layer
{
	/// Called when this slot is the cause of a wakeup, is passed a new handle to this slot's
	/// waiter (`waiter`) and the result of the previous async operation.
	fn advance(&mut self, object: ObjectHandle, stack: StackPush, result: usize) -> Option<usize>;
}

/// A handle to an async stack that is only allowed to push to the stack (cannot pop or access values)
pub struct StackPush<'a,'b: 'a>
{
	_pd: PhantomData<&'a mut AsyncStack<'b>>,
	stack: *mut AsyncStack<'b>,
}

const SLOT_STACK_SIZE_WORDS: usize = 256 / 8;
type AsyncStack<'a> = ::stack_dst::StackA<dyn Layer+'a, [usize; SLOT_STACK_SIZE_WORDS]>;

/// A single async slot (i.e. a single async operation as requested by the user)
///
/// This should have a stable address for the life of the operation (as hardware will have a handle to it)
/// TODO: How can this be ensured? Force a self-borrow? It must be _safe_, which implies that this has to be used via ARefBorrow?
#[derive(Default)]
pub struct Object<'a>
{
	inner: ObjectInner,
	/// Async call stack
	stack: Spinlock<AsyncStack<'a>>,
}
#[cfg(_false)]
pub struct VoidObject
{
	inner: ObjectInner,
}
#[derive(Default)]
struct ObjectInner
{
	/// A pointer to the currently-active waiter object. If null, there's no active sleeper
	waiter: atomic::AtomicPtr<SleepObject<'static>>,
	/// Result of the async operation, set to indicate it's time for the operation to advance
	// TODO: Could this be a AtomicUsize with a magic value (-1?) indicating empty?
	result: Spinlock<Option<usize>>,
}
/// A handle to an Object (as passed to state updates)
#[derive(Clone)]
pub struct ObjectHandle
{
	ptr: ::core::ptr::NonNull<ObjectInner>,
}
unsafe impl Send for ObjectHandle {
}

/// Top-level waiter object (see module-level documentation)
pub struct Waiter<'h, 'a: 'h>
{
	sleeper: SleepObject<'static>,
	handles: &'h [&'h Object<'a>],
}
impl<'h, 'a: 'h> !Sync for Waiter<'h, 'a> {}

// ----------
// Object
// ----------
impl<'a> Object<'a>
{
	pub fn get_handle(&self) -> ObjectHandle {
		// TODO: UNSAFE! How can this ensure that the pointer is valid as long as this object exists?
		ObjectHandle {
			ptr: ::core::ptr::NonNull::new(&self.inner as *const _ as *mut _).unwrap(),
			}
	}
	pub fn get_stack<'s>(&'s mut self) -> StackPush<'s, 'a> {
		StackPush::new(self.stack.get_mut())
	}

	fn check(&self, idx: usize) -> Option<usize> {
		let mut res = self.inner.result.lock().take();
		while let Some(res_val) = res
		{
			// There's a result waiting!
			let mut stack_lh = self.stack.lock();

			// If there's no handler, return.
			if stack_lh.is_empty()
			{
				log_debug!("check_one: {} result {:#x}", idx, res_val);
				return Some(res_val);
			}
			log_debug!("check_one: {} advance {:#x}", idx, res_val);

			// Stack has an entry, so pass the value on to that entry.
			let test_ptr: *mut ();
			res = {
				// - Magic handle that only allows pushing to this (append-only) stack
				let (magic_handle, top) = StackPush::new_with_top(&mut stack_lh);
				test_ptr = top as *mut _ as *mut ();

				top.advance(self.get_handle(), magic_handle, res_val)
				};
			// If this returns non-None, then pop and continue
			if res.is_some()
			{
				assert!( !stack_lh.is_empty() );
				assert_eq!( stack_lh.top_mut().unwrap() as *mut _ as *mut (), test_ptr, "Non-None return from async advance, but it also registered callbacks." );
				stack_lh.pop();
			}
		}
		None
	}
}
impl ObjectHandle
{
	pub fn signal(&self, result: usize) {
		// SAFE: (TODO TODO NOT SAFE)
		let obj = unsafe { self.ptr.as_ref() };

		{
			let mut lh = obj.result.lock();
			if lh.is_some() {
				panic!("Signalling already-signalled object");
			}
			*lh = Some(result);
		}
		
		let waiter = obj.waiter.load(Ordering::SeqCst);
		if waiter != ::core::ptr::null_mut() {
			// SAFE: This pointer's validity is maintained by this module, scoped to the call stack
			unsafe { (*waiter).signal() };
		}
	}
}

// ----------
// Waiter
// ----------
impl<'h, 'a: 'h> Waiter<'h, 'a>
{
	/// Construct a new waiter with `count` slots
	pub fn new(handles: &'h [&'h Object<'a>]) -> Waiter<'h, 'a>
	{
		Waiter {
			// SAFE: TODO: This isn't safe, the sleep object must have a stable pointer to be safe
			sleeper: unsafe { SleepObject::new("Waiter") },
			handles: handles,
			}
	}
	/// Check for an event (returning `None` if there is no pending event)
	pub fn check_one(&self) -> Option<WaitResult>
	{
		for (idx,h) in Iterator::enumerate(self.handles.iter())
		{
			if let Some(res_val) = h.check(idx)
			{
				return Some(WaitResult { slot: idx, result: res_val });
			}
		}

		None
	}

	/// Waits for an operation to complete
	/// Returns `None` if all operations are complete.
	pub fn wait_one(&self) -> Option<WaitResult>
	{
		// 0. Check if there are any active waiters in the list
		// - If not, return None
		if self.handles.iter().all(|h| h.stack.lock().is_empty())
		{
			// - All of the handles had empty stacks
			return None;
		}

		// 1. Register the sleep handle with the individual handles.
		let _reg = HandleSleepReg::new(&self.sleeper, self.handles);
		// 2. Loop until any operation completes
		loop
		{
			if let Some(rv) = self.check_one()
			{
				return Some(rv);
			}
			self.sleeper.wait();
		}
	}
}

// -----------
// StackPush
// -----------
impl<'a, 'b: 'a> StackPush<'a, 'b>
{
	fn new(stack: &'a mut AsyncStack<'b>) -> StackPush<'a, 'b> {
		StackPush { _pd: PhantomData, stack }
	}
	/// Construct a new instance as well as get a pointer to the top of the stack
	fn new_with_top(stack: &'a mut AsyncStack<'b>) -> (StackPush<'a, 'b>, &'a mut (dyn Layer+'b))
	{
		(StackPush { _pd: PhantomData, stack }, stack.top_mut().expect("new_with_top"), )
	}
	/// Push onto the stack
	pub fn push<T: Layer + 'b>(&mut self, v: T) -> Result<(), T>
	{
		// SAFE: The rule with this type is that it is ONLY allowed to push.
		unsafe {
			(*self.stack).push(v)
		}
	}
	pub fn push_closure<F: FnMut(ObjectHandle, StackPush, usize)->Option<usize> + 'b>(&mut self, f: F) -> Result<(), ()>
	{
		self.push(ClosureLayer(f)).map_err(|_| ())
	}
}

/// Result from an async operation
pub struct WaitResult {
	/// Index of the slot that completed
	pub slot: usize,
	/// Result value from the operation
	pub result: usize,
}


// --------------------------------------------------------------------
// HELPERS
// --------------------------------------------------------------------

//
struct HandleSleepReg<'a, 'h, 'ha: 'h>
{
	so: &'a SleepObject<'a>,
	handles: &'h [&'h Object<'ha>],
}
impl<'a, 'h, 'ha: 'h> HandleSleepReg<'a, 'h, 'ha>
{
	fn new(so: &'a SleepObject, handles: &'h [&'h Object<'ha>]) -> Result<HandleSleepReg<'a, 'h, 'ha>, usize>
	{
		// Create return structure before starting to register
		let rv = HandleSleepReg {
			so: so,
			handles: handles,
			};
		// Try to register, returning Err(index) if it fails
		for (i,h) in handles.iter().enumerate()
		{
			let ex = h.inner.waiter.compare_exchange(::core::ptr::null_mut(), so as *const _ as *mut _, Ordering::SeqCst, Ordering::Relaxed);
			if let Err(ex) = ex
			{
				// Uh-oh, something else is waiting on this?
				log_error!("HandleSleepReg::new - Entry {} {:p} was already registered with {:p}, this is trying {:p}",
					i, h, ex, so);
				return Err( i );
			}
		}
		Ok(rv)
	}
}
impl<'a, 'h, 'ha: 'h> ::core::ops::Drop for HandleSleepReg<'a, 'h, 'ha>
{
	fn drop(&mut self)
	{
		// Deregister all handles (if they're set to this object)
		for h in self.handles
		{
			match h.inner.waiter.compare_exchange(self.so as *const _ as *mut _, ::core::ptr::null_mut(), Ordering::SeqCst, Ordering::Relaxed)
			{
			Ok(_) => {},
			Err(_) => {},	// Could be NULL, or maybe it was another registration?
			}
		}
	}
}

// --------------------------------------------------------------------
// Trait Impls
// --------------------------------------------------------------------
struct ClosureLayer<F>(pub F);
impl<F: FnMut(ObjectHandle, StackPush, usize)->Option<usize>> Layer for ClosureLayer<F>
{
	fn advance(&mut self, object: ObjectHandle, stack: StackPush, result: usize) -> Option<usize> {
		(self.0)(object, stack, result)
	}
}
