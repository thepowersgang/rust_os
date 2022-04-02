//!
//!
//!
use core::ops;
use core::sync::atomic::{AtomicU8, Ordering};

pub struct SpinlockInner
{
	flag: AtomicU8,
}

fn cur_cpu() -> usize { 0 }

impl SpinlockInner
{
	pub const fn new() -> SpinlockInner {
		SpinlockInner {
			flag: AtomicU8::new(0),
			}
	}
	pub fn inner_lock(&self) {
		let my_id = cur_cpu() as u8 + 1;
		if self.flag.load(Ordering::Acquire) == 1 {
			panic!("Double-lock");
		}
		// Set flag to my_id if zero, loop otherwise
		while self.flag.compare_exchange(0, my_id, Ordering::Acquire, Ordering::Acquire).is_err() {
		}
	}
	pub fn try_inner_lock_cpu(&self) -> bool {
		let my_id = cur_cpu() as u8 + 1;
		if self.flag.load(Ordering::Acquire) == my_id {
			false
		}
		else {
			while self.flag.compare_exchange(0, my_id, Ordering::Acquire, Ordering::Acquire).is_err() {
			}
			true
		}
	}
	pub unsafe fn inner_release(&self)
	{
		self.flag.store(0, Ordering::Release)
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

pub fn hold_interrupts() -> HeldInterrupts {
	// TODO: Ensure that interrupts were on to begin with
	// SAFE: TODO
	unsafe {
		stop_interrupts();
	}
	HeldInterrupts
}

pub unsafe fn test_and_stop_interrupts() -> bool {
	false
}
pub unsafe fn stop_interrupts() {
	::core::arch::asm!("msr DAIFSet, #0x7");
}
pub unsafe fn start_interrupts() {
	::core::arch::asm!("msr DAIFClr, #0x7");
}

