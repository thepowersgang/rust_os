//
//
//
//! Synchonisation primitives
use core::ops;
use core::sync::atomic::{AtomicUsize,Ordering};
use core::cell::UnsafeCell;

/// Primitive Mutex
pub struct Mutex<T>(AtomicUsize, UnsafeCell<T>);
unsafe impl<T: Send> Sync for Mutex<T> {}
unsafe impl<T: Send> Send for Mutex<T> {}
impl<T> Mutex<T>
{
	pub const fn new(v: T) -> Mutex<T> {
		Mutex( AtomicUsize::new(1), UnsafeCell::new(v) )
	}

	pub fn lock(&self) -> HeldMutex<T> {
		if self.0.fetch_sub(1, Ordering::Acquire) == 1 {
			HeldMutex { _ptr: self, }
		}
		else {
			panic!("TODO: Acquire Mutex when contended");
		}
	}

	pub fn unwrap(self) -> T {
		assert_eq!( self.0.load(Ordering::Relaxed), 0 );
		self.1.into_inner()
	}

	/// UNSAFE: User needs to ensure that resources are no longer borrowed
	pub unsafe fn unlock(&self) {
		if self.0.fetch_add(1, Ordering::Release) != 0 {
			panic!("TODO: Release Mutex when contended");
		}
	}
}

pub struct HeldMutex<'a, T: 'a> {
	_ptr: &'a Mutex<T>,
}
impl<'a, T: 'a> ops::Deref for HeldMutex<'a, T>
{
	type Target = T;

	fn deref(&self) -> &T {
		// SAFE: HeldMutex controls this memory
		unsafe { &*self._ptr.1.get() }
	}
}
impl<'a, T: 'a> ops::DerefMut for HeldMutex<'a, T>
{
	fn deref_mut(&mut self) -> &mut T {
		// SAFE: HeldMutex controls this memory
		unsafe { &mut *self._ptr.1.get() }
	}
}
impl<'a, T: 'a> ops::Drop for HeldMutex<'a, T>
{
	fn drop(&mut self) {
		if self._ptr.0.fetch_add(1, Ordering::Release) != 0 {
			panic!("TODO: Release Mutex when contended");
		}
	}
}

pub fn futex_wait(addr: &AtomicUsize, sleep_if_val: usize)
{
	// SAFE: Assumed
	unsafe {
		syscall!(CORE_FUTEX_SLEEP, addr as *const _ as usize, sleep_if_val);
	}
}
pub fn futex_wake(addr: &AtomicUsize, num_to_wake: usize)
{
	// SAFE: Assumed
	unsafe {
		syscall!(CORE_FUTEX_WAKE, addr as *const _ as usize, num_to_wake);
	}
}

