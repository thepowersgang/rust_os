//
//
//
///
use core::ops;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize,Ordering};

pub struct Mutex<T>
{
	locked: AtomicUsize,
	data: UnsafeCell<T>,
}
unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

// NOTE: Unlock code requires these exact values
/// Lock is unlocked
const STATE_UNLOCKED   : usize = 0;
/// Locked, with nothing waiting
const STATE_UNCONTENDED: usize = 1;
/// Locked, and maybe something waiting
const STATE_CONTENDED  : usize = 2;

impl<T> Mutex<T>
{
	pub const fn new(val: T) -> Mutex<T> {
		Mutex {
			locked: ::core::sync::atomic::ATOMIC_USIZE_INIT,
			data: UnsafeCell::new(val),
		}
	}

	pub fn lock(&self) -> HeldMutex<T> {
		let mut cur = self.locked.compare_and_swap(STATE_UNLOCKED, STATE_UNCONTENDED, Ordering::Acquire);
		// If it wasn't locked, contention has happened
		if cur != STATE_UNLOCKED
		{
			// If the lock was uncontended
			if cur != STATE_CONTENDED {
				// Mark it as contended
				cur = self.locked.swap(STATE_CONTENDED, Ordering::Acquire);
			}
			// While the last seen value wasn't "unlocked"
			while cur != STATE_UNLOCKED {
				// Go to sleep if still contended when wait starts
				//::syscalls::sync::futex_wait(&self.locked, STATE_CONTENDED);
				panic!("TODO: Mutex::lock() - futex_wait()");
				cur = self.locked.swap(STATE_CONTENDED, Ordering::Acquire)
			}
		}
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
		// If "downgrading" the state wasn't from uncontended (i.e. it's from contended, or a bug and unlocked)
		if self.ptr.locked.fetch_sub(1, Ordering::Release) != STATE_UNCONTENDED {
			// - Set to unlocked state
			self.ptr.locked.store(STATE_UNLOCKED, Ordering::Release);
			// - And wake one waiter
			//::syscalls::sync::futex_wake(&self.ptr.locked, 1);
			panic!("TODO: Release contended mutex");
		}
		// In unlocked state
	}
}

