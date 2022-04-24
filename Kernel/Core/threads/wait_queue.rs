// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads/wait_queue.rs
//! Thread wait queue

use super::thread::RunState;
use super::ThreadList;

use super::{get_cur_thread,rel_cur_thread,reschedule};
use super::s_runnable_threads;

/// A list of waiting threads, can be woken one at a time, or all at once
pub struct WaitQueue
{
	list: ThreadList,
}

impl WaitQueue
{
	pub const fn new() -> WaitQueue {
 		WaitQueue {
			list: super::THREADLIST_INIT
			}
	}
	
	#[doc(hidden)]
	//#[not_safe(irq,taskswitch)]
	pub fn wait_int(&mut self) -> crate::arch::sync::HeldInterrupts
	{
		log_trace!("WaitQueue::wait({:p})", self);
		
		// - Prevent interrupts from firing while we mess with the thread
		let irq_lock = crate::arch::sync::hold_interrupts();
		
		// 1. Lock global list?
		let mut cur = get_cur_thread();
		// - Keep rawptr kicking around for debug purposes
		cur.set_state( RunState::ListWait(self as *mut _ as *const _) );
		// 2. Push current thread into waiting list
		self.list.push(cur);
		
		irq_lock
	}
	
	/// Wait the current thread on this queue, releasng the passed lock before actually sleeping
	///
	/// If this queue is accessed via `lock_handle`, use the `waitqueue_wait_ext` macro instead. E.g.
	/// ```
	/// waitqueue_wait_ext!(lock_handle, queue);
	/// ```
	pub fn wait<'a,T:Send>(&mut self, lock_handle: crate::arch::sync::HeldSpinlock<'a,T>)
	{
		let irq_lock = self.wait_int();
		// 3. Unlock protector, and allow IRQs once more
		::core::mem::drop(lock_handle);
		::core::mem::drop(irq_lock);
			
		// 4. Reschedule, and should return with state changed to run
		reschedule();
		
		// DEBUG check
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
	pub fn wake_one(&mut self) -> Option<super::ThreadID>
	{
		match self.list.pop()
		{
		Some(mut t) => {
			let tid = t.get_tid();
			log_trace!("WaitQueue::wake_one({:p}): Waking TID{}", self, tid);
			t.set_state( RunState::Runnable );
			let _irq_lock = crate::arch::sync::hold_interrupts();
			s_runnable_threads.lock().push(t);
			Some(tid)
			},
		None => None,
		}
	}
}

impl Default for WaitQueue
{
	fn default() -> WaitQueue {
		WaitQueue::new()
	}
}

