// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/semaphore.rs
//! Thread blocking semaphore type
//use prelude::*;
use crate::sync::Spinlock;

/// Semaphore
pub struct Semaphore
{
	max_value: isize,
	
	internals: Spinlock<Inner>,
}

struct Inner
{
	value: isize,
	wait_queue: crate::threads::WaitQueue,
	//signal_queue: ::threads::WaitQueue,
}

impl Semaphore
{
	pub const fn new(init_val: isize, max_val: isize) -> Semaphore {
		//assert!(max_val > 0, "Maximum semaphore value must be >0 ({})", max_val);
		//assert!(init_val <= max_val, "Initial value must be <= max ({} > {})", init_val, max_val);
		Semaphore {
			max_value: max_val,
			internals: Spinlock::new( Inner {
				value: init_val,
				wait_queue: crate::threads::WaitQueue::new(),
				//signal_queue: crate::threads::WaitQueue::new(),
				} ),
		}
	}
	
	pub fn acquire(&self) {
		let mut lh = self.internals.lock();
		if lh.value < 1 {
			log_trace!("acquire: value={} < 1, sleeping", lh.value);
			waitqueue_wait_ext!(lh, .wait_queue);
		}
		else {
			lh.value -= 1;
		}
	}
	pub fn release(&self) {
		let mut lh = self.internals.lock();
		if lh.wait_queue.has_waiter() {
			lh.wait_queue.wake_one();
		}
		else if lh.value < self.max_value {
			lh.value += 1;
		}
		else {
			todo!("Sleep on signal");
		}
	}
}

