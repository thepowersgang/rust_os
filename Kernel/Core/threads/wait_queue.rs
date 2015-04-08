// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads/wait_queue.rs
//! Thread wait queue
use _common::*;

use super::thread::RunState;
use super::ThreadList;

use super::{get_cur_thread,rel_cur_thread,reschedule};
use super::s_runnable_threads;

#[doc(hidden)]
pub const WAITQUEUE_INIT: WaitQueue = WaitQueue { list: super::THREADLIST_INIT };

/// A list of waiting threads, can be woken one at a time, or all at once
pub struct WaitQueue
{
	list: ThreadList,
}

impl WaitQueue
{
	pub fn new() -> WaitQueue {
		WAITQUEUE_INIT
	}
	
	/// Wait the current thread on this queue, releasng the passed lock before actually sleeping
	// TODO: Rewrite such that HeldSpinlock<WaitQueue> can be passed in?
	pub fn wait<'a,T:Send>(&mut self, lock_handle: ::arch::sync::HeldSpinlock<'a,T>)
	{
		log_trace!("WaitQueue::wait(...)");
		// - Prevent interrupts from firing while we mess with the thread
		let _irq_lock = ::arch::sync::hold_interrupts();
		
		// 1. Lock global list?
		let mut cur = get_cur_thread();
		// - Keep rawptr kicking around for debug purposes
		cur.set_state( RunState::ListWait(self as *mut _ as *const _) );
		// 2. Push current thread into waiting list
		self.list.push(cur);
		// 3. Unlock protector, and allow IRQs once more
		::core::mem::drop(lock_handle);
		::core::mem::drop(_irq_lock);
		// 4. Reschedule, and should return with state changed to run
		reschedule();
		
		let cur = get_cur_thread();
		cur.assert_active();
		rel_cur_thread(cur);
	}
	/// Returns true if there is a thread waiting on the list
        pub fn has_waiter(&self) -> bool
        {
                ! self.list.empty()
        }
	/// Wake a single thread waiting on this queue
	pub fn wake_one(&mut self)
	{
		log_trace!("WaitQueue::wake_one()");
		match self.list.pop()
		{
		Some(mut t) => {
			t.set_state( RunState::Runnable );
                        let _irq_lock = ::arch::sync::hold_interrupts();
			s_runnable_threads.lock().push(t);
			},
		None => {}
		}
	}
}

