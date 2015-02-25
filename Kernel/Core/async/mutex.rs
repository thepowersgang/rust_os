
use _common::*;
use super::EventWait;
use arch::sync::Spinlock;
use core::cell::UnsafeCell;

pub struct Mutex<T: Send>
{
	locked: Spinlock<bool>,
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
		unimplemented!()
	}
	
	pub fn try_lock(&self) -> Option<HeldMutex<T>>
	{
		unimplemented!()
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

