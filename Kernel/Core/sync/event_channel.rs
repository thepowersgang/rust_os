// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/event_channel.rs
//! Sleeping primitive for that wakes a thread when signalled
#[allow(unused_imports)]
use crate::prelude::*;
use crate::sync::Spinlock;
use crate::threads::WaitQueue;
use core::cell::UnsafeCell;
use core::sync::atomic::Ordering;

/// EventChannel controlling object
pub struct EventChannel
{
	lock: Spinlock<bool>,
	// Separate from the lock because WaitQueue::wait() takes a bool lock
	queue: UnsafeCell< WaitQueue >,
	pending_wakes: ::core::sync::atomic::AtomicUsize,
}
unsafe impl Sync for EventChannel {}
impl Default for EventChannel {
	fn default() -> Self {
		EventChannel::new()
	}
}

impl EventChannel
{
	/// Constant initialiser for EventChannel
	pub const fn new() -> EventChannel {
		EventChannel {
			lock: Spinlock::new( false ),
			queue: UnsafeCell::new( WaitQueue::new() ),
			pending_wakes: ::core::sync::atomic::AtomicUsize::new(0),
			}
	}
	
	/// Sleep until an event
	pub fn sleep(&self) {
		// SAFE: Queue is only accessed with the lock held
		unsafe {
			let mut lh = self.lock.lock();
			// If an event has fired, clear it and don't sleep.
			if *lh == true {
				log_trace!("EventChannel::sleep() - ready");
				*lh = false;
			}
			// Otherwise, sleep until after event
			else {
				log_trace!("EventChannel::sleep() - wait");
				(*self.queue.get()).wait(lh);
			}
		}
	}
	
	/// Clear any pending event
	pub fn clear(&self) {
		*self.lock.lock() = false;
	}

	/// Post the event
	//#[tag_safe(irq)]	// SAFE: Handles case of lock being held by CPU
	pub fn post(&self) {
		log_trace!("EventChannel::post()");
		// Attempt to lock (failing if the CPU already holds the lock)
		if let Some(mut lh) = self.lock.try_lock_cpu() {
			let mut count = 1;
			loop
			{
				// SAFE: Only called when lock is held
				unsafe {
					let q = &mut *self.queue.get();
					
					// Wake a sleeper, or set a flag preventing next sleep
					while count > 0 && q.has_waiter() {
						q.wake_one();
						count -= 1;
					}
					if count > 0 {
						*lh = true;
					}
				}
				
				// Release the lock, and check any pending wake requests
				// - Should not be racy, as it's a single-CPU action
				// - IRQ here will inc count
				drop(lh);
				// - IRQ here will lock successfully
				count = self.pending_wakes.swap(0, Ordering::Release);
				if count == 0 {
					break;
				}
				lh = self.lock.lock();
			}
		} else {
			// Set a flag
			self.pending_wakes.fetch_add(1, Ordering::Acquire);
		}
	}
}


