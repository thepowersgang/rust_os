// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads/sleep_object.rs
//! Sleep object
use prelude::*;
use core::ops;
use super::thread::{Thread, RunState};
use super::s_runnable_threads;

/// An object on which a thread can sleep, woken by various event sources
///
/// This object should not be moved while references are active
pub struct SleepObject
{
	name: &'static str,
	inner: ::sync::Spinlock< SleepObjectInner >,
}
impl_fmt! {
	Debug(self,f) for SleepObject {{
		let lh = self.inner.lock();
		write!(f, "SleepObject(\"{}\" {} refs, flag={})", self.name, lh.reference_count, lh.flag)
	}}
}
#[derive(Default)]
struct SleepObjectInner
{
	flag: bool,
	reference_count: usize,
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
		//log_trace!("SleepObject::wait {:p} '{}'", self, self.name);
		
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
	#[tag_safe(irq)]
	#[allow(not_tagged_safe)]	// Holds an IRQ lock
	pub fn signal(&self)
	{
		//log_trace!("SleepObject::signal {:p} '{}'", self, self.name);
		
		let _irq_lock = ::sync::hold_interrupts();
		let mut lh = self.inner.lock();
		// 1. Check for a waiter
		if let Some(mut t) = lh.thread.take()
		{
			t.set_state( RunState::Runnable );
			s_runnable_threads.lock().push(t);
		}
		else
		{
			lh.flag = true;
		}
	}
	
	/// Obtain a reference to the sleep object
	///
	/// NOTE: After this is called, self must not move
	pub fn get_ref(&self) -> SleepObjectRef {
		self.inner.lock().reference_count += 1;
		SleepObjectRef {
			obj: self as *const _,
		}
	}
}

impl ops::Drop for SleepObject
{
	fn drop(&mut self)
	{
		let lh = self.inner.lock();
		assert!(lh.reference_count == 0, "Sleep object being dropped while references are active");
	}
}

impl SleepObjectRef
{
	/// Checks if this reference points to the passed object
	pub fn is_from(&self, obj: &SleepObject) -> bool {
		self.obj == obj as *const _
	}
}
impl ops::Deref for SleepObjectRef
{
	type Target = SleepObject;
	
	fn deref(&self) -> &SleepObject {
		// SAFE: Reference counting ensures that this pointer is valid.
		unsafe { &*self.obj }   // > ASSUMPTION: The SleepObject doesn't move after it's borrowed
	}
}

impl ops::Drop for SleepObjectRef
{
	fn drop(&mut self)
	{
		// SAFE: Should still be valid
		let mut lh = unsafe { (*self.obj).inner.lock() };
		assert!(lh.reference_count > 0, "Sleep object's reference count is zero when dropping a reference");
		lh.reference_count -= 1;
	}
}

