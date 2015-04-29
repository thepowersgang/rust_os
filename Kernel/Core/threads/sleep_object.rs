// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads/sleep_object.rs
//! Sleep object
use _common::*;
use super::thread::{Thread, RunState};
use super::s_runnable_threads;

/// An object on which a thread can sleep, woken by various event sources
pub struct SleepObject
{
	name: &'static str,
	inner: ::sync::Spinlock< SleepObjectInner >,
}
#[derive(Default)]
struct SleepObjectInner
{
	flag: bool,
	thread: Option<Box<Thread>>,
}

/// Referece to an active sleep object
pub struct SleepObjectRef
{
	obj: *const SleepObject,
}
unsafe impl ::core::marker::Send for SleepObjectRef {}

impl SleepObject
{
	/// Create a new sleep object
	pub fn new(name: &'static str) -> SleepObject
	{
		SleepObject {
			name: name,
			inner: Default::default(),
		}
	}
	
	/// Wait the current thread on this object
	pub fn wait(&self)
	{
		log_trace!("SleepObject::wait {:p} '{}'", self, self.name);
		
		let irql = ::sync::hold_interrupts();
		let mut lh = self.inner.lock();
		assert!( lh.thread.is_none(), "A thread is already sleeping on object {:p} '{}'", self, self.name );
		
		if lh.flag == false
		{
			let mut cur = super::get_cur_thread();
			cur.run_state = RunState::Sleep(self as *const _);
			lh.thread = Some(cur);
			
			::core::mem::drop(lh);
			::core::mem::drop(irql);
			
			super::reschedule();
			
			let cur = super::get_cur_thread();
			assert!( !is!(cur.run_state, RunState::Sleep(_)) );
			assert!( is!(cur.run_state, RunState::Runnable) );
			super::rel_cur_thread(cur);
		}
		else
		{
			lh.flag = false;
		}
	}
	
	/// Signal this sleep object (waking threads)
	pub fn signal(&self)
	{
		log_trace!("SleepObject::signal {:p} '{}'", self, self.name);
		
		let mut lh = self.inner.lock();
		// 1. Check for a waiter
		if let Some(mut t) = lh.thread.take()
		{
			t.set_state( RunState::Runnable );
                        let _irq_lock = ::sync::hold_interrupts();
			s_runnable_threads.lock().push(t);
		}
		else
		{
			lh.flag = true;
		}
	}
	
	/// Obtain a reference to the sleep object
	pub fn get_ref(&self) -> SleepObjectRef {
		SleepObjectRef {
			obj: self as *const _,
		}
	}
}

impl ::core::ops::Deref for SleepObjectRef
{
	type Target = SleepObject;
	
	fn deref(&self) -> &SleepObject {
		unsafe { &*self.obj }
	}
}

