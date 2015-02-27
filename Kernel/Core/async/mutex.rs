// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/mutex.rs
///! Asynchronous Mutex
use _common::*;
use super::EventWait;
use core::cell::UnsafeCell;
use core::atomic::{AtomicBool,Ordering};

pub struct Mutex<T: Send>
{
	locked: AtomicBool,
	event: super::EventSource,
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
			event: super::EventSource::new(),
			data: UnsafeCell::new(data),
		}
	}
	
	pub fn try_lock(&self) -> Option<HeldMutex<T>>
	{
		if self.locked.swap(true, Ordering::Acquire)
		{
			Some(HeldMutex { __lock: self })
		}
		else
		{	
			None
		}
	}
	
	/// Asynchronously lock the mutex
	pub fn async_lock<F: FnOnce(&mut EventWait, HeldMutex<T>)>(&self, f: F) -> EventWait
	{
		unimplemented!()
	}
}

#[unsafe_destructor]
impl<'a,T: Send + 'a> ::core::ops::Drop for HeldMutex<'a, T>
{
	fn drop(&mut self)
	{
		
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

