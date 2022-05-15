//!

use ::core::cell::UnsafeCell;
//use ::core::sync::atomic::{self,AtomicBool,Ordering};
use ::core::task;
use ::core::pin::Pin;

pub struct Mutex<T>
{
	lock_state: crate::sync::Spinlock<LockState>,
	wait_state: crate::sync::Mutex<WaitState>,
	data: UnsafeCell<T>,
}
struct LockState {
	locked: bool,
	next_ticket: usize,
}
struct WaitState {
	cur_ticket: usize,
	waiters: super::helpers::WakerQueue,
}
unsafe impl<T: Send> Sync for Mutex<T> {}
unsafe impl<T: Send> Send for Mutex<T> {}

impl<T> Mutex<T>
{
	/// Construct a new unsafe mutex
	pub const fn new(data: T) -> Mutex<T>
	{
		Mutex {
			lock_state: crate::sync::Spinlock::new(LockState {
				locked: false,
				next_ticket: 0,
				}),
			wait_state: crate::sync::Mutex::new(WaitState {
				cur_ticket: !0,
				waiters: super::helpers::WakerQueue::new(),
				}),
			data: UnsafeCell::new(data),
		}
	}
}
impl<T: Default> Default for Mutex<T> {
	fn default() -> Self {
		Self::new(T::default())
	}
}

impl<T: Send> Mutex<T>
{
	/// Attempt to lock the mutex (returning None on failure)
	pub fn try_lock(&self) -> Option<HeldMutex<T>>
	{
		let mut lh = self.lock_state.lock();
		if !lh.locked {
			lh.locked = true;
			log_trace!("async::Mutex<{}>::try_lock - success", type_name!(T));
			Some(HeldMutex { __lock: self })
		}
		else {
			None
		}
	}
	
	/// Asynchronously lock the mutex
	pub fn async_lock(&self) -> Waiter<T>
	{
		Waiter {
			lock: self,
			state: WaiterState::Init,
		}
	}
}

// 
//
//

/// Wait object for the async mutex
pub struct Waiter<'a,T: Send+'a>
{
	lock: &'a Mutex<T>,
	state: WaiterState,
}
#[derive(Debug)]
enum WaiterState
{
	/// Step 1: Register this (once we're pinned, we should never be leaked?)
	Init,
	/// Step 2: Waiting for our ticket to come up
	Waiting(usize),
	/// Step 3: Completed
	Complete
}
impl<'a, T: Send + 'a> ::core::future::Future for Waiter<'a, T>
{
	type Output = HeldMutex<'a, T>;
	fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context) -> task::Poll<Self::Output> {
		let lock = self.lock;
		if let WaiterState::Complete = self.state {
			panic!("");
		}
		if let WaiterState::Init = self.state {
			let mut lh = lock.lock_state.lock();
			// Short-circuit successful lock
			if !lh.locked {
				lh.locked = true;
				self.state = WaiterState::Complete;
				return task::Poll::Ready(HeldMutex { __lock: self.lock });
			}

			// Create a handle that will be the next one woken
			let ticket = lh.next_ticket; lh.next_ticket += 1;
			self.state = WaiterState::Waiting(ticket);
		}

		// Currently waiting, keep sleeping until the current ticket is equal to our ticket
		if let WaiterState::Waiting(ticket) = self.state {
			let mut lh = lock.wait_state.lock();
			if lh.cur_ticket == ticket {
				self.state = WaiterState::Complete;
				return task::Poll::Ready(HeldMutex { __lock: self.lock });
			}
			lh.waiters.push(cx.waker());
			return task::Poll::Pending;
		}
		todo!("");
	}
}
impl<'a, T: Send + 'a> ::core::ops::Drop for Waiter<'a, T> {
	fn drop(&mut self) {
		match self.state
		{
		WaiterState::Init => {},
		WaiterState::Complete => {},
		WaiterState::Waiting(_ticket) => {
			// Since we're on the wait queue, we need to cede our position.
			// - Could either do that by block-waiting, or by adding our index to a list of early-dropped indexes
			todo!("Cede position in wait queue");
			}
		}
	}
}


// --------------------------------------------------------------------
// HeldMutex
// --------------------------------------------------------------------
/// Lock handle
pub struct HeldMutex<'a, T: Send + 'a> {
	__lock: &'a Mutex<T>,
}

impl<'a,T: Send + 'a> ::core::ops::Drop for HeldMutex<'a, T>
{
	fn drop(&mut self)
	{
		let mut lh_w = self.__lock.wait_state.lock();
		let mut lh_l = self.__lock.lock_state.lock();
		if lh_w.cur_ticket == lh_l.next_ticket {
			lh_l.locked = false;
			log_trace!("futures::HeldMutex<{}>::drop - release", type_name!(T));
		}
		else {
			drop(lh_l);
			lh_w.cur_ticket += 1;
			// If a thread was woken, they now own this lock
			if lh_w.waiters.wake_one() {
				log_trace!("futures::HeldMutex<{}>::drop - yield", type_name!(T));
			}
			else {
				log_trace!("futures::HeldMutex<{}>::drop - yield to nobody?", type_name!(T));
			}
		}
	}
}

impl<'a,T: Send + 'a> ::core::ops::Deref for HeldMutex<'a, T>
{
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: & to handle, hence no &mut possible
		unsafe { &*self.__lock.data.get() }
	}
}

impl<'a,T: Send + 'a> ::core::ops::DerefMut for HeldMutex<'a, T>
{
	fn deref_mut(&mut self) -> &mut T {
		// SAFE: &mut to handle, hence &mut is safe
		unsafe { &mut *self.__lock.data.get() }
	}
}

