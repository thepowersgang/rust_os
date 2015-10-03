/*
 */
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool,Ordering};

pub struct Spinlock<T>
{
	flag: AtomicBool,
	value: UnsafeCell<T>,
}
unsafe impl<T: Send> Sync for Spinlock<T> {}
unsafe impl<T: Send> Send for Spinlock<T> {}
impl<T: Default> Default for Spinlock<T> {
	fn default() -> Spinlock<T> {
		Spinlock::new(Default::default())
	}
}

pub struct HeldSpinlock<'a, T: 'a> {
	_ptr: &'a Spinlock<T>
}

fn acquire(flag: &AtomicBool) {
	//super::puts("lock()\n");
	//super::puts("lock() flag = ");
	//super::puth(flag.load(Ordering::Relaxed) as u64);
	//super::puts(", flag = ");
	//super::puth(flag as *const AtomicBool as usize as u64);
	//super::puts("\n");
	
	while flag.swap(true, Ordering::Acquire) {
		// ...
	}
	//super::puts("- Locked\n");
}

impl<T> Spinlock<T>
{
	pub const fn new(v: T) -> Spinlock<T> {
		Spinlock {
			flag: AtomicBool::new(false),
			value: UnsafeCell::new(v),
		}
	}

	pub fn lock(&self) -> HeldSpinlock<T> {
		acquire(&self.flag);
		HeldSpinlock {
			_ptr: self
			}
	}
	pub fn try_lock_cpu(&self) -> Option<HeldSpinlock<T>> {
		// TODO: Ensure that this CPU isn't holding the lock
		if self.flag.swap(true, Ordering::Acquire) == false {
			Some( HeldSpinlock { _ptr: self } )
		}
		else {
			None
		}
	}
}

impl<'a, T: 'a> ::core::ops::Deref for HeldSpinlock<'a, T>
{
	type Target = T;

	fn deref(&self) -> &T {
		// SAFE: Lock is held
		unsafe { &*self._ptr.value.get() }
	}
}
impl<'a, T: 'a> ::core::ops::DerefMut for HeldSpinlock<'a, T>
{
	fn deref_mut(&mut self) -> &mut T {
		// SAFE: Lock is held
		unsafe { &mut *self._ptr.value.get() }
	}
}
impl<'a, T: 'a> ::core::ops::Drop for HeldSpinlock<'a, T> {
	fn drop(&mut self) {
		let v = self._ptr.flag.swap(false, Ordering::Release);
		assert!(v);
	}
}


pub struct HeldInterrupts;
pub fn hold_interrupts() -> HeldInterrupts {
	HeldInterrupts
}
impl ::core::ops::Drop for HeldInterrupts {
	fn drop(&mut self) {
	}
}

