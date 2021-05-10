/*
 */
use core::sync::atomic::{AtomicBool,Ordering};

pub struct SpinlockInner
{
	flag: AtomicBool,
}
impl SpinlockInner
{
	pub const fn new() -> SpinlockInner {
		SpinlockInner { flag: AtomicBool::new(false) }
	}

	pub fn inner_lock(&self)
	{
		while self.flag.swap(true, Ordering::Acquire) {
			// TODO: Once SMP is a thing, this should spin.
			super::puts("Contented lock!");
			loop {}
		}
	}
	pub unsafe fn inner_release(&self)
	{
		assert!( self.flag.load(Ordering::Relaxed) );
		self.flag.store(false, Ordering::Release);
	}

	pub fn try_inner_lock_cpu(&self) -> bool
	{
		// TODO: Ensure that this CPU isn't holding the lock
		if self.flag.swap(true, Ordering::Acquire) == false {
			true
		}
		else {
			false
		}
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
pub fn stop_interrupts() {
	// SAFE: Correct inline assembly
	unsafe { asm!("cpsid if"); }
}
pub fn start_interrupts() {
	// SAFE: Correct inline assembly
	unsafe { asm!("cpsie if"); }
}

