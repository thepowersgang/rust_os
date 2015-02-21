//
//
//
use lib::LazyStatic;
use core::marker::{Send, Sync};
use core::ops::Fn;

/// A standard mutex
pub struct Mutex<T: Send>
{
	pub locked_held: ::sync::Spinlock<bool>,
	pub queue: ::core::cell::UnsafeCell<::threads::WaitQueue>,
	pub val: ::core::cell::UnsafeCell<T>,
}
// Mutexes are inherently sync
unsafe impl<T: Send> Sync for Mutex<T> { }
unsafe impl<T: Send> Send for Mutex<T> { }

/// Lock handle on a mutex
pub struct HeldMutex<'lock,T:'lock+Send>
{
	lock: &'lock Mutex<T>
}

/// A lazily populated mutex (contained type is allocated on the heap upon first lock)
pub struct LazyMutex<T: Send>(pub Mutex<LazyStatic<T>>);

impl<T: Send> Mutex<T>
{
	/*
	pub fn new(val: T) -> Mutex<T> {
		Mutex {
			locked_held: spinlock_init!(false),
			queue: ::threads::WAITQUEUE_INIT,
			val: val,
		}
	}
	*/
	
	fn queue(&self) -> &mut ::threads::WaitQueue
	{
		unsafe { &mut *self.queue.get() }
	}
	
	/// Lock the mutex
	#[inline(never)]
	pub fn lock(&self) -> HeldMutex<T> {
		{
			// Check the held status of the mutex
			// - Spinlock protected variable
			let mut held = self.locked_held.lock();
			if *held != false
			{
				// If mutex is locked, then wait for it to be unlocked
				// - ThreadList::wait will release the passed spinlock
				self.queue().wait(held);
			}
			else
			{
				*held = true;
			}
		}
		::core::atomic::fence(::core::atomic::Ordering::Acquire);
		return HeldMutex { lock: self };
	}
	/// Release the mutex
	fn unlock(&self) {
		::core::atomic::fence(::core::atomic::Ordering::Release);
		let mut held = self.locked_held.lock();
		if self.queue().has_waiter()
		{
			self.queue().wake_one();
			// *held is still true, as the newly woken thread now owns the mutex
		}
		else
		{
			*held = false;
		}
	}
}

impl<T: Send> LazyMutex<T>
{
	/// Lock and (if required) initialise using init_fcn
	pub fn lock_init<Fcn: Fn()->T>(&self, init_fcn: Fcn) -> HeldMutex<LazyStatic<T>>
	{
		let mut lh = self.0.lock();
		lh.prep(init_fcn);
		lh
	}
	
	pub fn init<Fcn: Fn()->T>(&self, init_fcn: Fcn)
	{
		let mut lh = self.0.lock();
		lh.prep(init_fcn);
	}
	pub fn lock(&self) -> HeldMutex<LazyStatic<T>>
	{
		self.0.lock()
	}
}

#[unsafe_destructor]
impl<'lock,T:Send> ::core::ops::Drop for HeldMutex<'lock,T>
{
	/// Unlock on drop of HeldMutex
	fn drop(&mut self) {
		self.lock.unlock();
	}
}
impl<'lock,T:Send> ::core::ops::Deref for HeldMutex<'lock,T>
{
	type Target = T;
	fn deref<'a>(&'a self) -> &'a T {
		unsafe { &*self.lock.val.get() }
	}
}
impl<'lock,T:Send> ::core::ops::DerefMut for HeldMutex<'lock,T>
{
	fn deref_mut<'a>(&'a mut self) -> &'a mut T {
		unsafe { &mut *self.lock.val.get() }
	}
}

#[macro_export]
macro_rules! mutex_init{ ($val:expr) => (::sync::mutex::Mutex{
	locked_held: spinlock_init!(false),
	queue: ::core::cell::UnsafeCell { value: ::threads::WAITQUEUE_INIT },
	val: ::core::cell::UnsafeCell{ value: $val },
	}) }
macro_rules! lazymutex_init{
	() => {::sync::mutex::LazyMutex(mutex_init!( ::lib::LazyStatic(None) ))}
}

// vim: ft=rust

