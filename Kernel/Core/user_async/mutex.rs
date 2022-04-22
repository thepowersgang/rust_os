// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/mutex.rs
//! Asynchronous Mutex.
//!
//! Provides an asynchonous mutex type, for use with the async IO framework

#[allow(unused_imports)]
use crate::prelude::*;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool,Ordering};
use core::fmt;
use super::PrimitiveWaiter;

// NOTES:
// This should support:
// - Blocking waits (by deferring to a new async structure)
// - try_lock()
// - async_lock()
//  > Returns a handle registered to be the next lock handle
//  > If this handle is leaked, the lock deadlocks (understandably)
//  > When dropped, this handle will wait until the mutex is yielded to it before it instantly yields

/// Asynchronous mutex type
pub struct Mutex<T: Send>
{
	locked: AtomicBool,
	waiters: super::sequential_queue::Source,
	data: UnsafeCell<T>,
}
unsafe impl<T: Send> Sync for Mutex<T> {}
unsafe impl<T: Send> Send for Mutex<T> {}

/// Wait object for the async mutex
pub struct Waiter<'a,T: Send+'a>
{
	lock: &'a Mutex<T>,
	state: WaitState<'a>,
}
#[derive(Debug)]
enum WaitState<'a>
{
	Sleep(super::sequential_queue::Waiter<'a>),
	Complete,
	Consumed
}

/// Lock handle
pub struct HeldMutex<'a, T: Send + 'a>
{
	__lock: &'a Mutex<T>,
}

impl<T: Send> Mutex<T>
{
	/// Construct a new unsafe mutex
	pub fn new(data: T) -> Mutex<T>
	{
		Mutex {
			locked: AtomicBool::new(false),
			waiters: super::sequential_queue::Source::new(),
			data: UnsafeCell::new(data),
		}
	}
	
	/// Attempt to lock the mutex (returning None on failure)
	pub fn try_lock(&self) -> Option<HeldMutex<T>>
	{
		if self.locked.swap(true, Ordering::Acquire) == false
		{
			log_trace!("async::Mutex<{}>::try_lock - success", type_name!(T));
			Some(HeldMutex { __lock: self })
		}
		else
		{	
			None
		}
	}
	
	/// Asynchronously lock the mutex
	pub fn async_lock(&self) -> Waiter<T>
	{
		// Short-circuit successful lock
		if self.locked.swap(true, Ordering::Acquire) == false
		{
			log_trace!("async::Mutex<{}>::async_lock - success", type_name!(T));
			// A complete handle is a lock handle that doesn't yet fully exist
			Waiter::new_complete(self)
		}
		else
		{
			log_trace!("async::Mutex<{}>::async_lock - wait", type_name!(T));
			// Create a handle that will be the next one woken
			Waiter::new_sleep(self, &self.waiters)
		}
	}
}


// --
// Waiter : A handle representing a position in the queue
// --
impl<'a, T: Send> Waiter<'a,T>
{
	fn new_sleep<'b>(lock: &'b Mutex<T>, queue: &'b super::sequential_queue::Source) -> Waiter<'b, T> {
		Waiter { lock: lock, state: WaitState::Sleep(queue.wait_on()) }
	}
	fn new_complete(lock: &Mutex<T>) -> Waiter<T> {
		Waiter { lock: lock, state: WaitState::Complete }
	}
	
	pub fn take_lock(&mut self) -> HeldMutex<'a, T> {
		match self.state
		{
		WaitState::Sleep(_) => panic!("Waiter<{}>::take_lock - Still sleeping", type_name!(T)),
		WaitState::Complete => {
			self.state = WaitState::Consumed;
			HeldMutex { __lock: self.lock }
			},
		WaitState::Consumed => panic!("Waiter<{}>::take_lock - Already consumed", type_name!(T)),
		}
	}
}
impl<'a, T: Send> ::core::ops::Drop for Waiter<'a, T>
{
	fn drop(&mut self) {
		match self.state
		{
		WaitState::Sleep(_) => todo!("Either panic or wait until the lock is released"),
		WaitState::Complete => {
			// Allocate a lock handle then let it drop, 
			let _ = HeldMutex { __lock: self.lock };
			},
		WaitState::Consumed => {},
		}
	}
}

impl<'a, T: Send> fmt::Debug for Waiter<'a,T>
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "async::Mutex<{}>::Waiter(state={:?})", type_name!(T), self.state)
	}
}

impl<'a, T: Send> PrimitiveWaiter for Waiter<'a,T>
{
	fn is_complete(&self) -> bool {
		log_debug!("Waiter<{}>::is_complete - self.state={:?}", type_name!(T), self.state);
		match self.state
		{
		WaitState::Sleep(ref obj) => obj.is_complete(),
		WaitState::Complete => true,
		WaitState::Consumed => true,
		}
	}
	fn poll(&self) -> bool {
		match self.state
		{
		WaitState::Sleep(ref obj) => obj.poll(),
		WaitState::Complete => true,
		WaitState::Consumed => false,
		}
	}
	fn run_completion(&mut self) {
		self.state = match self.state
			{
			WaitState::Sleep(ref mut obj) => {
				assert!(obj.is_complete());
				obj.run_completion();
				WaitState::Complete
				},
			_ => return,
			};
	}
	fn bind_signal(&mut self, sleeper: &mut crate::threads::SleepObject) -> bool {
		// If not in Sleep state, force a poll (which will complete instantly)
		match self.state
		{
		WaitState::Sleep(ref mut obj) => obj.bind_signal(sleeper),
		WaitState::Complete => false,
		WaitState::Consumed => {
			log_warning!("Waiter<{}>::bind_signal called on consumed", type_name!(T));
			true
			},
		}
	}
	fn unbind_signal(&mut self) {
		match self.state
		{
		WaitState::Sleep(ref mut obj) => obj.unbind_signal(),
		_ => {},
		}
	}

}

impl<'a,T: Send + 'a> ::core::ops::Drop for HeldMutex<'a, T>
{
	fn drop(&mut self)
	{
		if self.__lock.waiters.wake_one()
		{
			// If a thread was woken, they now own this lock
			log_trace!("async::HeldMutex<{}>::drop - yield", type_name!(T));
		}
		else
		{
			log_trace!("async::HeldMutex<{}>::async_lock - release", type_name!(T));
			self.__lock.locked.store(false, Ordering::Release);
		}
	}
}

impl<'a,T: Send + 'a> ::core::ops::Deref for HeldMutex<'a, T>
{
	type Target = T;
	fn deref(&self) -> &T
	{
		// SAFE: & to handle, hence no &mut possible
		unsafe { &*self.__lock.data.get() }
	}
}

impl<'a,T: Send + 'a> ::core::ops::DerefMut for HeldMutex<'a, T>
{
	fn deref_mut(&mut self) -> &mut T
	{
		// SAFE: &mut to handle, hence &mut is safe
		unsafe { &mut *self.__lock.data.get() }
	}
}

