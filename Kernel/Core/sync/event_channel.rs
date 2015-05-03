// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/event_channel.rs
//! Sleeping primitive for that wakes a thread when signalled
use prelude::*;
use sync::Spinlock;
use threads::WaitQueue;
use core::cell::UnsafeCell;

/// EventChannel controlling object
pub struct EventChannel
{
	lock: Spinlock<bool>,
	queue: UnsafeCell< WaitQueue >,
}
unsafe impl Sync for EventChannel {}

/// Static initialiser for EventChannel
pub const EVENTCHANNEL_INIT: EventChannel = EventChannel {
	lock: spinlock_init!( false ),
	queue: UnsafeCell{value: ::threads::WAITQUEUE_INIT}
	};

impl EventChannel
{
	pub fn new() -> EventChannel {
		EVENTCHANNEL_INIT
	}
	
	/// Sleep until an event
	pub fn sleep(&self) {
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
	pub fn post(&self) {
		unsafe {
			let mut lh = self.lock.lock();
			let q = &mut *self.queue.get();
			
			// Wake a sleeper, or set a flag preventing next sleep
			if q.has_waiter() {
				q.wake_one();
			}
			else {
				*lh = true;
			}
		}
	}
}


