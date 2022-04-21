// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/queue.rs
//! Thread-safe (optionally unbounded) generic queue
#[allow(unused_imports)]
use crate::prelude::*;
use crate::sync::Spinlock;
use core::cell::UnsafeCell;
use crate::sync::Mutex;
use crate::lib::VecDeque;

pub struct Queue<T>
{
	lock: Spinlock<bool>,
	// Separate from the lock because WaitQueue::wait() takes a bool lock
	queue: UnsafeCell< crate::threads::WaitQueue >,
	data: Mutex<VecDeque<T>>,
}

unsafe impl<T: Send> Sync for Queue<T> {
}
unsafe impl<T: Send> Send for Queue<T> {
}

impl<T> Queue<T>
{
	pub const fn new_const() -> Self
	{
		Queue {
			lock: Spinlock::new(false),
			queue: UnsafeCell::new(crate::threads::WaitQueue::new()),
			data: Mutex::new(VecDeque::new_const()),
			}
	}
}

impl<T: Send> Queue<T>
{
	pub fn push(&self, v: T) {
		// 1. Push the value.
		self.data.lock().push_back(v);
		// 2. Set and signal
		if let Some(mut lh) = self.lock.try_lock_cpu() {
			*lh = true;
			// SAFE: Locked by the above lock
			unsafe {
				(*self.queue.get()).wake_one();
			}
		}
		else {
			// TODO: How can this be handled? What will wake the CPU?
		}
	}
	pub fn wait_pop(&self) -> T {
		loop
		{
			// 1. Check for pending (in the queue)
			if let Some(v) = self.data.lock().pop_front() {
				return v;
			}

			// 2. if nothing, lock spinlock and check if the inner flag is set
			let mut lh = self.lock.lock();
			//  - If set, loop
			if *lh {
				*lh = false;
				continue ;
			}
			//  - Else, sleep
			else {
				// SAFE: This lock controls this waiter.
				unsafe {
					(*self.queue.get()).wait(lh);
				}
			}
		}
	}
}

