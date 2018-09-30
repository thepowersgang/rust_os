// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/mutex.rs
//! Thread blocking Mutex type
#[allow(unused_imports)]
use prelude::*;
use core::ops;

/// A standard mutex (blocks the current thread when contended)
pub struct Mutex<T: Send>
{
	inner: ::sync::Spinlock<MutexInner>,
	val: ::core::cell::UnsafeCell<T>,
}

#[doc(hidden)]
pub struct MutexInner
{
	held: bool,
	holder: ::threads::ThreadID,
	queue: ::threads::WaitQueue,
}

// Mutexes are inherently sync
unsafe impl<T: Send> Sync for Mutex<T> { }
unsafe impl<T: Send> Send for Mutex<T> { }

/// Lock handle on a mutex
pub struct HeldMutex<'lock,T:'lock+Send>
{
	lock: &'lock Mutex<T>
}

/// A lazily populated mutex (must be initialised on/before first lock)
pub struct LazyMutex<T: Send>(Mutex<Option<T>>);

/// Wrapper handle for a held LazyMutex
pub struct HeldLazyMutex<'a, T: Send+'a>( HeldMutex<'a, Option<T>> );

impl<T: Send> Mutex<T>
{
	/// Construct a new mutex-protected value
	pub const fn new(val: T) -> Mutex<T> {
		Mutex {
			inner: ::sync::Spinlock::new(MutexInner {
				held: false,
				holder: 0,
				queue: ::threads::WaitQueue::new(),
				}),
			val: ::core::cell::UnsafeCell::new(val),
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
				assert!(lh.holder != ::threads::get_thread_id(), "Recursive lock of {}", type_name!(Self));
				// If mutex is locked, then wait for it to be unlocked
				// - ThreadList::wait will release the passed spinlock
				waitqueue_wait_ext!(lh, .queue);
				// lh.queue.wait(lh);	// << Trips borrowck
				self.inner.lock().holder = ::threads::get_thread_id();
			}
			else
			{
				lh.held = true;
				lh.holder = ::threads::get_thread_id();
			}
		}
		::core::sync::atomic::fence(::core::sync::atomic::Ordering::Acquire);
		return HeldMutex { lock: self };
	}
	/// Release the mutex
	fn unlock(&self) {
		::core::sync::atomic::fence(::core::sync::atomic::Ordering::Release);
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
	pub const fn new() -> LazyMutex<T> {
		LazyMutex( Mutex::new(None) )
	}
	
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
		else {
			log_notice!("LazyMutex::init() called multiple times: T={}", type_name!(T));
		}
	}
	/// Lock the lazy mutex
	pub fn lock(&self) -> HeldLazyMutex<T>
	{
		let lh = self.0.lock();
		assert!(lh.is_some(), "Locking an uninitialised LazyMutex<{}>", type_name!(T));
		HeldLazyMutex( lh )
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
		// SAFE: & to the handle, means that mutable alias is impossible
		unsafe { &*self.lock.val.get() }
	}
}
impl<'lock,T:Send> ops::DerefMut for HeldMutex<'lock,T>
{
	fn deref_mut<'a>(&'a mut self) -> &'a mut T {
		// SAFE: &mut to the handle, means that &mut to inner is safe
		unsafe { &mut *self.lock.val.get() }
	}
}

impl<'l,T:Send> ops::Deref for HeldLazyMutex<'l,T>
{
	type Target = T;
	fn deref(&self) -> &T {
		self.0.as_ref().expect("Derefencing an uninitialised LazyMutex")
	}
}
impl<'l,T:Send> ops::DerefMut for HeldLazyMutex<'l,T>
{
	fn deref_mut(&mut self) -> &mut T {
		self.0.as_mut().expect("Derefencing an uninitialised LazyMutex")
	}
}

// vim: ft=rust

