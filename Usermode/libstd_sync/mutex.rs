//
//
//
///
use core::ops;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool,Ordering};

pub struct Mutex<T>
{
	locked: AtomicBool,
	data: UnsafeCell<T>,
}
unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T>
{
	pub const fn new(val: T) -> Mutex<T> {
		Mutex {
			locked: ::core::sync::atomic::ATOMIC_BOOL_INIT,
			data: UnsafeCell::new(val),
		}
	}

	pub fn lock(&self) -> HeldMutex<T> {
		let v = self.locked.swap(true, Ordering::Acquire);
		// TODO: Actually lock
		assert_eq!(v, false);
		HeldMutex { ptr: self }
	}
}

pub struct HeldMutex<'a, T: 'a>
{
	ptr: &'a Mutex<T>,
}

impl<'a, T: 'a> ops::Deref for HeldMutex<'a, T> {
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: & to handle means that & to data is safe
		unsafe { &*self.ptr.data.get() }
	}
}
impl<'a, T: 'a> ops::DerefMut for HeldMutex<'a, T> {
	fn deref_mut(&mut self) -> &mut T {
		// SAFE: &mut to handle means that &mut to data is safe
		unsafe { &mut *self.ptr.data.get() }
	}
}
impl<'a, T> ::core::ops::Drop for HeldMutex<'a, T> {
	fn drop(&mut self) {
		self.ptr.locked.store(false, Ordering::Release);
	}
}

