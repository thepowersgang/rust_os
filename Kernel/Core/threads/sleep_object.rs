
use _common::*;
use super::thread::{Thread, RunState};

/// An object on which a thread can sleep, woken by various event sources
pub struct SleepObject
{
	name: &'static str,
	thread: Option<Box<Thread>>,
}

pub struct SleepObjectRef
{
	obj: *const SleepObject,
}

impl SleepObject
{
	pub fn new(name: &'static str) -> SleepObject
	{
		SleepObject {
			name: name,
			thread: None,
		}
	}
	
	pub fn wait(&mut self)
	{
		let mut cur = super::get_cur_thread();
		cur.run_state = RunState::Sleep(self as *mut _ as *const _);
		self.thread = Some(cur);
		super::reschedule();
		
		let cur = self.thread.take().unwrap();
		assert!( !is!(cur.run_state, RunState::Sleep(_)) );
		assert!( is!(cur.run_state, RunState::Runnable) );
		super::rel_cur_thread(cur);
	}
	
	pub fn signal(&self)
	{
		
	}
	
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

