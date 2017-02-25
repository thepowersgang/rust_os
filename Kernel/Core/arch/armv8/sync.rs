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

fn cur_cpu() -> usize { 0 }

impl<T> Spinlock<T>
{
	pub const fn new(v: T) -> Spinlock<T> {
		Spinlock {
			flag: AtomicU8::new(0),
			data: UnsafeCell::new(v),
			}
	}
	pub fn lock(&self) -> HeldSpinlock<T> {
		let my_id = cur_cpu() as u8 + 1;
		if self.flag.load(Ordering::Acquire) == 1 {
			panic!("Double-lock");
		}
		// Set flag to my_id if zero, loop otherwise
		while self.flag.compare_exchange(0, my_id, Ordering::Acquire, Ordering::Acquire).is_err() {
		}
		HeldSpinlock(self)
	}
	pub fn try_lock_cpu(&self) -> Option<HeldSpinlock<T>> {
		let my_id = cur_cpu() as u8 + 1;
		if self.flag.load(Ordering::Acquire) == my_id {
			None
		}
		else {
			while self.flag.compare_exchange(0, my_id, Ordering::Acquire, Ordering::Acquire).is_err() {
			}
			Some(HeldSpinlock(self))
		}
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

