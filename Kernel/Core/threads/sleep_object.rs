// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads/sleep_object.rs
//! Sleep object
use _common::*;
use super::thread::{Thread, RunState};

/// An object on which a thread can sleep, woken by various event sources
pub struct SleepObject
{
	name: &'static str,
	thread: Option<Box<Thread>>,
}

/// Referece to an active sleep object
pub struct SleepObjectRef
{
	obj: *const SleepObject,
}

//pub const SLEEPOBJECT_INIT: SleepObject = SleepObject { name: "", thread: None };

impl SleepObject
{
	/// Create a new sleep object
	pub fn new(name: &'static str) -> SleepObject
	{
		SleepObject {
			name: name,
			thread: None,
		}
	}
	
	/// Wait the current thread on this object
	pub fn wait(&mut self)
	{
		log_trace!("SleepObject::wait '{}'", self.name);
		let mut cur = super::get_cur_thread();
		cur.run_state = RunState::Sleep(self as *mut _ as *const _);
		self.thread = Some(cur);
		super::reschedule();
		
		let cur = self.thread.take().unwrap();
		assert!( !is!(cur.run_state, RunState::Sleep(_)) );
		assert!( is!(cur.run_state, RunState::Runnable) );
		super::rel_cur_thread(cur);
	}
	
	/// Signal this sleep object (waking threads)
	pub fn signal(&self)
	{
		todo!("SleepObject::signal '{}'", self.name);
	}
	
	/// Obtain a reference to the sleep object
	pub fn get_ref(&self) -> SleepObjectRef {
		SleepObjectRef {
			obj: self as *const _,
		}
	}
}

unsafe impl ::core::marker::Send for SleepObjectRef {}

impl ::core::ops::Deref for SleepObjectRef
{
	type Target = SleepObject;
	
	fn deref(&self) -> &SleepObject {
		unsafe { &*self.obj }
	}
}

