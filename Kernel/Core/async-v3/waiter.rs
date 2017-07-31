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
use prelude::*;
use threads::SleepObject;
use core::sync::atomic::{Ordering,AtomicUsize};
use core::marker::PhantomData;
use core::ops;
use sync::Spinlock;

/// Trait representing a saved piece of async state
pub trait Layer
{
	/// Called when this slot is the cause of a wakeup, is passed a new handle to this slot's
	/// waiter (`waiter`) and the result of the previous async operation.
	fn advance(&mut self, waiter: WaitHandle, result: usize) -> Option<usize>;
}

const SLOT_STACK_SIZE_WORDS: usize = 256 / 8;

#[derive(Default)]
/// A single slot
struct WaitSlot
{
	res: Spinlock<Option<usize>>,
	stack: Spinlock<::stack_dst::StackA<Layer, [usize; SLOT_STACK_SIZE_WORDS]>>,
}
/// Inner data useful to waiter handles
struct WaiterInner
{
	sleeper: SleepObject<'static>,
	handle_count: AtomicUsize,
}

/// Top-level waiter object (see module-level documentation)
pub struct Waiter<'a>
{
	// Indicate that this object logically stores a borrow to itself (when called via a &self method)
	_lock: PhantomData<::core::cell::Cell<&'a ()>>,
	inner: WaiterInner,
	slots: Vec<WaitSlot>,
}
impl<'a> !Sync for Waiter<'a> {}

impl<'a> Waiter<'a>
{
	/// Construct a new waiter with `count` slots
	pub fn new(count: usize) -> Waiter<'a>
	{
		Waiter {
			_lock: PhantomData,
			inner: WaiterInner {
				sleeper: SleepObject::new("Waiter"),
				handle_count: AtomicUsize::new(0),
				},
			slots: (0..count).map(|_| Default::default()).collect(),
			}
	}
	/// Obtain a handle to the waiter, prevents moving after call.
	pub fn get_handle(&'a self, slot: usize) -> WaitHandle
	{
		let v = self.inner.handle_count.fetch_add(1, Ordering::SeqCst);
		assert!( v != <usize>::max_value() );
		WaitHandle(&self.slots[slot], &self.inner)
	}
	/// Check for an event (returning `None` if there is no pending event)
	pub fn check_event(&'a self) -> Option<WaitResult>
	{
		for (idx,slot) in self.slots.iter().enumerate()
		{
			let mut opt_res = slot.res.lock().take();
			while let Some(v) = opt_res
			{
				let top_ptr = match slot.stack.lock().top_mut()
					{
					None =>  {
						// TODO: Mark slot as complete? Or should that be done via counting handles?
						return Some(WaitResult { slot: idx, result: v });
						},
					Some(p) => p as *mut Layer,
					};

				// Pass to the top item in the stack
				// SAFE: This requires that items are only ever popped/read in this context. This type can't be accessed concurrently (!Sync)
				opt_res = unsafe { (*top_ptr).advance(self.get_handle(idx), v) };
				if opt_res.is_some() {
					let mut lh = slot.stack.lock();
					assert_eq!( Some(top_ptr as *const _), lh.top().map(|x| x as *const _), "Operation was pushed when result returned from `advance`" );
					lh.pop();
				}
			}
		}

		None
	}
	/// Wait for an event to occur. May return `None` if there was an event that was handled further up the chain.
	pub fn wait_event(&'a self) -> Option<WaitResult>
	{
		self.inner.sleeper.wait();
		self.check_event()
	}
}
impl<'a> Drop for Waiter<'a>
{
	fn drop(&mut self)
	{
		// Ensure that there are no more handles out to this waiter.
		assert!(self.inner.handle_count.load(Ordering::SeqCst) == 0, "Waiter dropped while handles still exist");
	}
}

pub struct WaitHandle(*const WaitSlot, *const WaiterInner);
unsafe impl Send for WaitHandle {}
unsafe impl Sync for WaitHandle {}

impl WaitHandle
{
	pub fn wake(&mut self, result: usize)
	{
		// SAFE: Pointer stability ensured by self-borrow and lifetime ensured by reference count
		let mut lh = unsafe { (*self.0).res.lock() };
		if lh.is_some() {
			panic!("WaitHandle::wake - already signalled");
		}
		else {
			*lh = Some(result);
		}
		// SAFE: Pointer stability ensured by self-borrow and lifetime ensured by reference count
		unsafe { (*self.1).sleeper.signal(); }
	}

	pub fn push_state<T: Layer+'static>(&mut self, s: T)
	{
		// SAFE: Pointer stability ensured by self-borrow and lifetime ensured by reference count
		match unsafe { (*self.0).stack.lock().push(s) }
		{
		Ok(_) => {},
		Err(_) => panic!("Out of space when pushing to an async stack"),
		}
	}
}
impl ops::Drop for WaitHandle
{
	fn drop(&mut self) {
		// SAFE: Lifetime ensured by reference count
		unsafe {
			(*self.1).handle_count.fetch_sub(1, Ordering::SeqCst);
		}
	}
}

pub struct WaitResult {
	slot: usize,
	result: usize,
}


