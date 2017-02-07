
use core::cell::UnsafeCell;
use core::ops;

pub struct Spinlock<T>
{
	flag: UnsafeCell<u32>,
	data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for Spinlock<T> {}
unsafe impl<T: Send> Send for Spinlock<T> {}

impl<T> Spinlock<T>
{
	pub const fn new(v: T) -> Spinlock<T> {
		Spinlock {
			flag: UnsafeCell::new(0),
			data: UnsafeCell::new(v),
			}
	}
	pub fn lock(&self) -> HeldSpinlock<T> {
		todo!("");
	}
	pub fn try_lock_cpu(&self) -> Option<HeldSpinlock<T>> {
		todo!("");
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


pub struct HeldInterrupts;

pub fn hold_interrupts()->HeldInterrupts {
	HeldInterrupts
}
pub unsafe fn stop_interrupts() {
}
pub unsafe fn start_interrupts() {
}

