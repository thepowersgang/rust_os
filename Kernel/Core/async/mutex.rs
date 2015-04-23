// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/mutex.rs
//! Asynchronous Mutex.
//!
//! Provides an asynchonous mutex type, for use with the async IO framework
use _common::*;
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
	queue_wait: super::queue::Waiter<'a>,
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
		unimplemented!()
	}
}

impl<'a, T: Send> Waiter<'a,T>
{
	pub fn take_lock(&mut self) -> HeldMutex<'a, T> {
		unimplemented!()
	}
}

impl<'a, T: Send> fmt::Debug for Waiter<'a,T>
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "Mutex<{}>::Waiter", type_name!(T))
	}
}

impl<'a, T: Send> super::PrimitiveWaiter for Waiter<'a,T>
{
	fn poll(&self) -> bool {
		self.queue_wait.poll()
	}
	fn run_completion(&mut self) {
		self.queue_wait.run_completion();
		todo!("Mutex acquired... what do I do?");
	}
	fn bind_signal(&mut self, sleeper: &mut ::threads::SleepObject) -> bool {
		self.queue_wait.bind_signal(sleeper)
	}

}

#[unsafe_destructor]
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

