//!
use core::cell::UnsafeCell;
use core::ops;
use core::sync::atomic::{AtomicU8, Ordering};

pub struct Spinlock<T>
{
	flag: AtomicU8,
	data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for Spinlock<T> {}
unsafe impl<T: Send> Send for Spinlock<T> {}

impl<T> Spinlock<T>
{
	pub const fn new(v: T) -> Spinlock<T> {
		Spinlock {
			flag: AtomicU8::new(0),
			data: UnsafeCell::new(v),
			}
	}
	pub fn lock(&self) -> HeldSpinlock<T> {
		if self.flag.load(Ordering::Acquire) == 1 {
			panic!("Double-lock");
		}
		while self.flag.compare_exchange(0, 1, Ordering::Acquire, Ordering::Acquire).is_err() {
		}
		HeldSpinlock(self)
	}
	pub fn try_lock_cpu(&self) -> Option<HeldSpinlock<T>> {
		todo!("Spinlock::try_lock_cpu");
	}
}
impl<T> Default for Spinlock<T>
where
	T: Default
{
	fn default() -> Self {
		Spinlock::new(Default::default())
	}
}

pub struct HeldSpinlock<'a, T: 'a>(&'a Spinlock<T>);
impl<'a, T: 'a> ops::Deref for HeldSpinlock<'a, T>
{
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: Spinlock held
		unsafe { &*self.0.data.get() }
	}
}
impl<'a, T: 'a> ops::DerefMut for HeldSpinlock<'a, T>
{
	fn deref_mut(&mut self) -> &mut T {
		// SAFE: Spinlock held
		unsafe { &mut*self.0.data.get() }
	}
}
impl<'a, T: 'a> ops::Drop for HeldSpinlock<'a, T>
{
	fn drop(&mut self)
	{
		self.0.flag.store(0, Ordering::Release)
	}
}


pub struct HeldInterrupts;
impl ops::Drop for HeldInterrupts {
	fn drop(&mut self) {
		// SAFE: TODO
		unsafe {
			start_interrupts();
		}
	}
}

pub fn hold_interrupts()->HeldInterrupts {
	// SAFE: TODO
	unsafe {
		stop_interrupts();
	}
	HeldInterrupts
}
pub unsafe fn stop_interrupts() {
}
pub unsafe fn start_interrupts() {
}

