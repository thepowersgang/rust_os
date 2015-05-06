// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/mutex.rs
//! Asynchronous Mutex.
//!
//! Provides an asynchonous mutex type, for use with the async IO framework
use prelude::*;
use core::cell::UnsafeCell;
use core::atomic::{AtomicBool,Ordering};
use core::fmt;
use async::PrimitiveWaiter;

/// Asynchronous mutex type
pub struct Mutex<T: Send>
{
	locked: AtomicBool,
	waiters: super::queue::Source,
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
	Sleep(super::queue::Waiter<'a>),
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
			waiters: super::queue::Source::new(),
			data: UnsafeCell::new(data),
		}
	}
	
	/// Attempt to lock the mutex (returning None on failure)
	pub fn try_lock(&self) -> Option<HeldMutex<T>>
	{
		if self.locked.swap(true, Ordering::Acquire) == false
		{
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
		if self.locked.swap(true, Ordering::Acquire) == false
		{
			Waiter::new_complete(self)
		}
		else
		{
			Waiter::new_sleep(self, &self.waiters)
		}
	}
}

impl<'a, T: Send> Waiter<'a,T>
{
	fn new_sleep<'b>(lock: &'b Mutex<T>, queue: &'b super::queue::Source) -> Waiter<'b, T> {
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

impl<'a, T: Send> fmt::Debug for Waiter<'a,T>
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "Mutex<{}>::Waiter(state={:?})", type_name!(T), self.state)
	}
}

impl<'a, T: Send> super::PrimitiveWaiter for Waiter<'a,T>
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
				obj.run_completion();
				WaitState::Complete
				},
			_ => return,
			};
	}
	fn bind_signal(&mut self, sleeper: &mut ::threads::SleepObject) -> bool {
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
		}
		else
		{
			self.__lock.locked.store(false, Ordering::Release);
		}
	}
}

impl<'a,T: Send + 'a> ::core::ops::Deref for HeldMutex<'a, T>
{
	type Target = T;
	fn deref(&self) -> &T
	{
		unsafe { &*self.__lock.data.get() }
	}
}

impl<'a,T: Send + 'a> ::core::ops::DerefMut for HeldMutex<'a, T>
{
	fn deref_mut(&mut self) -> &mut T
	{
		unsafe { &mut *self.__lock.data.get() }
	}
}

