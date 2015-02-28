// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/mutex.rs
///! Asynchronous Mutex
use _common::*;
use super::Waiter;
use core::cell::UnsafeCell;
use core::atomic::{AtomicBool,Ordering};

pub struct Mutex<T: Send>
{
	locked: AtomicBool,
	waiters: super::QueueSource,
	data: UnsafeCell<T>,
}
unsafe impl<T: Send> Sync for Mutex<T> {}
unsafe impl<T: Send> Send for Mutex<T> {}

pub struct HeldMutex<'a, T: Send + 'a>
{
	__lock: &'a Mutex<T>,
}

impl<T: Send> Mutex<T>
{
	pub fn new(data: T) -> Mutex<T>
	{
		Mutex {
			locked: AtomicBool::new(false),
			waiters: super::QueueSource::new(),
			data: UnsafeCell::new(data),
		}
	}
	
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
	pub fn async_lock<'s, F: FnOnce(&mut Waiter, HeldMutex<'s,T>)>(&'s self, f: F) -> Waiter<'s>
	{
		unimplemented!()
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

