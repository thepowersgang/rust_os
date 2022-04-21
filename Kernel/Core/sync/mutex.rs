// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/mutex.rs
//! Thread blocking Mutex type
#[allow(unused_imports)]
use crate::prelude::*;
use core::ops;

/// A standard mutex (blocks the current thread when contended)
pub struct Mutex<T>
{
	inner: UnitMutex,
	val: ::core::cell::UnsafeCell<T>,
}

/// A mutex that controls no data (used as the non-generic portion of `Mutex<T>`)
struct UnitMutex
{
	inner: crate::sync::Spinlock<MutexInner>,
}

#[doc(hidden)]
struct MutexInner
{
	held: bool,
	holder: crate::threads::ThreadID,
	queue: crate::threads::WaitQueue,
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
pub struct LazyMutex<T>(Mutex<Option<T>>);

/// Wrapper handle for a held LazyMutex
pub struct HeldLazyMutex<'a, T: Send+'a>( HeldMutex<'a, Option<T>> );

impl UnitMutex
{
	pub const fn new() -> UnitMutex {
		UnitMutex {
			inner: crate::sync::Spinlock::new(MutexInner {
				held: false,
				holder: !0,
				queue: crate::threads::WaitQueue::new(),
				}),
			}
	}

	#[inline(never)]	// These are nice debugging points
	pub fn lock(&self, ty_name: &'static str) {
		Self::trace(self, ty_name, "lock");
		{
			// Check the held status of the mutex
			// - Spinlock protected variable
			let mut lh = self.inner.lock();
			if lh.held != false
			{
				assert!(lh.holder != crate::threads::get_thread_id(), "Recursive lock of {}", ty_name);
				// If mutex is locked, then wait for it to be unlocked
				// - ThreadList::wait will release the passed spinlock before sleeping. NOTE: It doesn't re-acquire the lock
				waitqueue_wait_ext!(lh, .queue);
				lh = self.inner.lock();
				assert!(lh.holder == crate::threads::get_thread_id(), "Invalid wakeup in lock of {}", ty_name);
			}
			else
			{
				lh.held = true;
				lh.holder = crate::threads::get_thread_id();
			}
		}
		Self::trace(self, ty_name, "lock - acquired");
		::core::sync::atomic::fence(::core::sync::atomic::Ordering::Acquire);
	}

	/// UNSAFE: Must only be called when the controlled resoure is being released
	#[inline(never)]	// These are nice debugging points
	pub unsafe fn unlock(&self, ty_name: &'static str) {
		Self::trace(self, ty_name, "unlock");
		::core::sync::atomic::fence(::core::sync::atomic::Ordering::Release);
		let mut lh = self.inner.lock();
		if let Some(tid) = lh.queue.wake_one()
		{
			lh.holder = tid;
			// *held is still true, as the newly woken thread now owns the mutex
		}
		else
		{
			lh.held = false;
		}
	}

	fn trace(ptr: *const UnitMutex, ty_name: &'static str, action: &str)
	{
		match ty_name
		{
		""
		//| "kernel::sync::mutex::Mutex<core::option::Option<kernel::lib::collections::vec_map::VecMap<usize, kernel::metadevs::storage::PhysicalVolumeInfo>>>"
			=> log_trace!("{}({:?})::{}", ty_name, ptr, action),
		_ => {},
		}
	}
}

impl<T> Mutex<T>
{
	/// Construct a new mutex-protected value
	pub const fn new(val: T) -> Mutex<T> {
		Mutex {
			inner: UnitMutex::new(),
			val: ::core::cell::UnsafeCell::new(val),
		}
	}
}
	
impl<T: Send> Mutex<T>
{
	/// Lock the mutex, blocking the current thread
	pub fn lock(&self) -> HeldMutex<T> {
		self.inner.lock(type_name!(Self));
		return HeldMutex { lock: self };
	}

	/// Obtain `&mut` to the contained data
	pub fn get_mut(&mut self) -> &mut T {
		// SAFE: Have exclusive access (`&mut self`)
		unsafe {
			&mut *self.val.get()
		}
	}
}

impl<T: Send+Default> Default for Mutex<T> {
	fn default() -> Mutex<T> {
		Mutex::new(<T as Default>::default())
	}
}

impl<T> LazyMutex<T>
{
	pub const fn new() -> LazyMutex<T> {
		LazyMutex( Mutex::new(None) )
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

/// Unlock on drop of HeldMutex
impl<'lock,T:Send> ops::Drop for HeldMutex<'lock,T>
{
	fn drop(&mut self) {
		// SAFE: This type controls the lock
		unsafe {
			self.lock.inner.unlock(type_name!(Mutex<T>));
		}
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

