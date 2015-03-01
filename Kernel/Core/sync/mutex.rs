// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/mutex.rs
//! Thread blocking Mutex type
use lib::LazyStatic;
use core::marker::{Send, Sync};
use core::ops::Fn;

/// A standard mutex (blocks the current thread when contended)
pub struct Mutex<T: Send>
{
	#[doc(hidden)]
	pub locked_held: ::sync::Spinlock<bool>,
	#[doc(hidden)]
	pub queue: ::core::cell::UnsafeCell<::threads::WaitQueue>,
	#[doc(hidden)]
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
	/// Construct a new mutex-protected value
	pub fn new(val: T) -> Mutex<T> {
		Mutex {
			locked_held: spinlock_init!(false),
			queue: ::core::cell::UnsafeCell { value: ::threads::WAITQUEUE_INIT },
			val: ::core::cell::UnsafeCell { value: val },
		}
	}
	
	fn queue(&self) -> &mut ::threads::WaitQueue
	{
		unsafe { &mut *self.queue.get() }
	}
	
	/// Lock the mutex, blocking the current thread
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
	
	/// Initialise the lazy mutex
	pub fn init<Fcn: Fn()->T>(&self, init_fcn: Fcn)
	{
		let mut lh = self.0.lock();
		lh.prep(init_fcn);
	}
	/// Lock the lazy mutex
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

/// Initialise a static Mutex
#[macro_export]
macro_rules! mutex_init{ ($val:expr) => (::sync::mutex::Mutex{
	locked_held: spinlock_init!(false),
	queue: ::core::cell::UnsafeCell { value: ::threads::WAITQUEUE_INIT },
	val: ::core::cell::UnsafeCell{ value: $val },
	}) }
/// Initialise a static LazyMutex
#[macro_export]
macro_rules! lazymutex_init{
	() => {::sync::mutex::LazyMutex(mutex_init!( ::lib::LazyStatic(None) ))}
}

// vim: ft=rust

