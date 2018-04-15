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
use prelude::*;
use threads::SleepObject;
use core::sync::atomic::{self, Ordering};
use core::marker::PhantomData;
use sync::Spinlock;

/// Trait representing a saved piece of async state
pub trait Layer
{
	/// Called when this slot is the cause of a wakeup, is passed a new handle to this slot's
	/// waiter (`waiter`) and the result of the previous async operation.
	fn advance(&mut self, object: ObjectHandle, stack: StackPush, result: usize) -> Option<usize>;
}

/// A handle to an async stack that is only allowed to push to the stack (cannot pop or access values)
pub struct StackPush<'a>
{
	_pd: PhantomData<&'a mut AsyncStack>,
	stack: *mut AsyncStack,
}

const SLOT_STACK_SIZE_WORDS: usize = 256 / 8;
type AsyncStack = ::stack_dst::StackA<Layer, [usize; SLOT_STACK_SIZE_WORDS]>;

#[derive(Default)]
/// A single async slot (i.e. a single async operation as requested by the user)
///
/// This should have a stable address for the life of the operation (as hardware will have a handle to it)
/// TODO: How can this be ensured? Force a self-borrow? It must be _safe_, which implies that this has to be used via ARefBorrow?
pub struct Object
{
	/// A pointer to the currently-active waiter object. If null, there's no active sleeper
	waiter: atomic::AtomicPtr<SleepObject<'static>>,
	/// Result of the async operation, set to indicate it's time for the operation to advance
	// TODO: Could this be a AtomicUsize with a magic value (-1?) indicating empty?
	result: Spinlock<Option<usize>>,
	/// Async call stack
	stack: Spinlock<AsyncStack>,
}
/// A handle to an Object (as passed to state updates)
pub struct ObjectHandle
{
	ptr: ::core::ptr::NonNull<Object>,
}
unsafe impl Send for ObjectHandle {
}

/// Top-level waiter object (see module-level documentation)
pub struct Waiter<'h>
{
	sleeper: SleepObject<'static>,
	handles: &'h [&'h Object],
}
impl<'a,'h> !Sync for Waiter<'h> {}

// ----------
// Object
// ----------
impl Object
{
	pub fn get_handle(&self) -> ObjectHandle {
		// TODO: UNSAFE! How can this ensure that the pointer is valid as long as this object exists?
		ObjectHandle {
			ptr: ::core::ptr::NonNull::new(self as *const _ as *mut _).unwrap(),
			}
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
impl<'h> Waiter<'h>
{
	/// Construct a new waiter with `count` slots
	pub fn new(handles: &'h [&'h Object]) -> Waiter<'h>
	{
		Waiter {
			sleeper: SleepObject::new("Waiter"),
			handles: handles,
			}
	}
	/// Check for an event (returning `None` if there is no pending event)
	pub fn check_one(&self) -> Option<WaitResult>
	{
		for (idx,h) in Iterator::enumerate(self.handles.iter())
		{
			let mut res = h.result.lock().take();
			while let Some(res_val) = res
			{
				// There's a result waiting!
				let mut stack_lh = h.stack.lock();

				// If there's no handler, return.
				if stack_lh.is_empty()
				{
					return Some(WaitResult { slot: idx, result: res_val });
				}

				// Stack has an entry, so pass the value on to that entry.
				let test_ptr: *mut ();
				res = {
					// - Magic handle that only allows pushing to this (append-only) stack
					let (magic_handle, top) = StackPush::new_with_top(&mut stack_lh);
					test_ptr = top as *mut _ as *mut ();

					top.advance(h.get_handle(), magic_handle, res_val)
					};
				// If this returns non-None, then pop and continue
				if res.is_some()
				{
					assert!( !stack_lh.is_empty() );
					assert_eq!( stack_lh.top_mut().unwrap() as *mut _ as *mut (), test_ptr, "Non-None return from async advance, but it also registered callbacks." );
					stack_lh.pop();
				}
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
impl<'a> StackPush<'a>
{
	/// Construct a new instance as well as get a pointer to the top of the stack
	fn new_with_top(stack: &mut AsyncStack) -> (StackPush, &mut Layer)
	{
		(StackPush { _pd: PhantomData, stack }, stack.top_mut().expect("new_with_top"), )
	}
	/// Push onto the stack
	pub fn push<T: Layer + 'static>(&mut self, v: T) -> Result<(), T>
	{
		// SAFE: The rule with this type is that it is ONLY allowed to push.
		unsafe {
			(*self.stack).push(v)
		}
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
struct HandleSleepReg<'a, 'h>
{
	so: &'a SleepObject<'a>,
	handles: &'h [&'h Object],
}
impl<'a, 'h> HandleSleepReg<'a, 'h>
{
	fn new(so: &'a SleepObject, handles: &'h [&'h Object]) -> Result<HandleSleepReg<'a, 'h>, usize>
	{
		// Create return structure before starting to register
		let rv = HandleSleepReg {
			so: so,
			handles: handles,
			};
		// Try to register, returning Err(index) if it fails
		for (i,h) in handles.iter().enumerate()
		{
			let ex = h.waiter.compare_and_swap(::core::ptr::null_mut(), so as *const _ as *mut _, Ordering::SeqCst);
			if ex != ::core::ptr::null_mut()
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
impl<'a, 'h> ::core::ops::Drop for HandleSleepReg<'a, 'h>
{
	fn drop(&mut self)
	{
		// Deregister all handles (if they're set to this object)
		for h in self.handles
		{
			h.waiter.compare_and_swap(self.so as *const _ as *mut _, ::core::ptr::null_mut(), Ordering::SeqCst);
		}
	}
}
