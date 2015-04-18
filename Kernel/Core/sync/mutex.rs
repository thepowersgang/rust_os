// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/mutex.rs
//! Thread blocking Mutex type
use lib::LazyStatic;
use core::option::Option::{self,Some,None};
use core::marker::{Send, Sync};
use core::ops::{self,FnOnce};
use core::default::Default;

/// A standard mutex (blocks the current thread when contended)
pub struct Mutex<T: Send>
{
	#[doc(hidden)]
	pub inner: ::sync::Spinlock<MutexInner>,
	#[doc(hidden)]
	pub val: ::core::cell::UnsafeCell<T>,
}

#[doc(hidden)]
pub struct MutexInner
{
	held: bool,
	queue: ::threads::WaitQueue,
}

#[doc(hidden)]
pub const MUTEX_INNER_INIT: MutexInner = MutexInner { held: false, queue: ::threads::WAITQUEUE_INIT };

// Mutexes are inherently sync
unsafe impl<T: Send> Sync for Mutex<T> { }
unsafe impl<T: Send> Send for Mutex<T> { }

/// Lock handle on a mutex
pub struct HeldMutex<'lock,T:'lock+Send>
{
	lock: &'lock Mutex<T>
}

/// A lazily populated mutex (must be initialised on/before first lock)
pub struct LazyMutex<T: Send>(pub Mutex<Option<T>>);

pub struct HeldLazyMutex<'a, T: Send+'a>( HeldMutex<'a, Option<T>> );

impl<T: Send> Mutex<T>
{
	/// Construct a new mutex-protected value
	pub fn new(val: T) -> Mutex<T> {
		Mutex {
			inner: spinlock_init!(MUTEX_INNER_INIT),
			val: ::core::cell::UnsafeCell { value: val },
		}
	}
	
	/// Lock the mutex, blocking the current thread
	#[inline(never)]
	pub fn lock(&self) -> HeldMutex<T> {
		{
			// Check the held status of the mutex
			// - Spinlock protected variable
			let mut lh = self.inner.lock();
			if lh.held != false
			{
				// If mutex is locked, then wait for it to be unlocked
				// - ThreadList::wait will release the passed spinlock
				waitqueue_wait_ext!(lh, queue);
				// lh.queue.wait(lh);	// << Trips borrowck
			}
			else
			{
				lh.held = true;
			}
		}
		::core::atomic::fence(::core::atomic::Ordering::Acquire);
		return HeldMutex { lock: self };
	}
	/// Release the mutex
	fn unlock(&self) {
		::core::atomic::fence(::core::atomic::Ordering::Release);
		let mut lh = self.inner.lock();
		if lh.queue.has_waiter()
		{
			lh.queue.wake_one();
			// *held is still true, as the newly woken thread now owns the mutex
		}
		else
		{
			lh.held = false;
		}
	}
}

impl<T: Send+Default> Default for Mutex<T> {
	fn default() -> Mutex<T> {
		Mutex::new(<T as Default>::default())
	}
}

impl<T: Send> LazyMutex<T>
{
	/// Lock and (if required) initialise using init_fcn
	pub fn lock_init<Fcn: FnOnce()->T>(&self, init_fcn: Fcn) -> HeldLazyMutex<T>
	{
		let mut lh = self.0.lock();
		if lh.is_none() {
			*lh = Some( init_fcn() );
		}
		HeldLazyMutex( lh )
	}
	
	/// Initialise the lazy mutex
	pub fn init<Fcn: FnOnce()->T>(&self, init_fcn: Fcn)
	{
		let mut lh = self.0.lock();
		if lh.is_none() {
			*lh = Some( init_fcn() );
		}
	}
	/// Lock the lazy mutex
	pub fn lock(&self) -> HeldLazyMutex<T>
	{
		HeldLazyMutex( self.0.lock() )
	}
}

impl<'lock,T:Send> ops::Drop for HeldMutex<'lock,T>
{
	/// Unlock on drop of HeldMutex
	fn drop(&mut self) {
		self.lock.unlock();
	}
}
impl<'lock,T:Send> ops::Deref for HeldMutex<'lock,T>
{
	type Target = T;
	fn deref<'a>(&'a self) -> &'a T {
		unsafe { &*self.lock.val.get() }
	}
}
impl<'lock,T:Send> ops::DerefMut for HeldMutex<'lock,T>
{
	fn deref_mut<'a>(&'a mut self) -> &'a mut T {
		unsafe { &mut *self.lock.val.get() }
	}
}

impl<'l,T:Send> ops::Deref for HeldLazyMutex<'l,T>
{
	type Target = T;
	fn deref(&self) -> &T {
		self.0.as_ref().unwrap()
	}
}
impl<'l,T:Send> ops::DerefMut for HeldLazyMutex<'l,T>
{
	fn deref_mut(&mut self) -> &mut T {
		self.0.as_mut().unwrap()
	}
}

/// Initialise a static Mutex
#[macro_export]
macro_rules! mutex_init{ ($val:expr) => ($crate::sync::mutex::Mutex{
	inner: spinlock_init!($crate::sync::mutex::MUTEX_INNER_INIT),
	val: ::core::cell::UnsafeCell{ value: $val },
	}) }
/// Initialise a static LazyMutex
#[macro_export]
macro_rules! lazymutex_init{
	() => {$crate::sync::mutex::LazyMutex(mutex_init!( None ))}
}

// vim: ft=rust

