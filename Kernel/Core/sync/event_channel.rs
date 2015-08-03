// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/event_channel.rs
//! Sleeping primitive for that wakes a thread when signalled
use prelude::*;
use sync::Spinlock;
use threads::WaitQueue;
use core::cell::UnsafeCell;
use core::atomic::Ordering;

/// EventChannel controlling object
pub struct EventChannel
{
	lock: Spinlock<bool>,
	// Separate from the lock because WaitQueue::wait() takes a bool lock
	queue: UnsafeCell< WaitQueue >,
	pending_wakes: ::core::atomic::AtomicUsize,
}
unsafe impl Sync for EventChannel {}

/// Static initialiser for EventChannel
pub const EVENTCHANNEL_INIT: EventChannel = EventChannel {
	lock: Spinlock::new( false ),
	queue: UnsafeCell::new( ::threads::WAITQUEUE_INIT ),
	pending_wakes: ::core::atomic::ATOMIC_USIZE_INIT,
	};

impl EventChannel
{
	pub fn new() -> EventChannel {
		EVENTCHANNEL_INIT
	}
	
	/// Sleep until an event
	pub fn sleep(&self) {
		// SAFE: Queue is only accessed with the lock held
		unsafe {
			let mut lh = self.lock.lock();
			// If an event has fired, clear it and don't sleep.
			if *lh == true {
				*lh = false;
			}
			// Otherwise, sleep until after event
			else {
				(*self.queue.get()).wait(lh);
			}
		}
	}
	
	/// Post the event
	//#[tag_safe(irq)]	// SAFE: Handles case of lock being held by CPU
	pub fn post(&self) {
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


