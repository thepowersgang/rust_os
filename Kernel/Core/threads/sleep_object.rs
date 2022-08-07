// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads/sleep_object.rs
//! Sleep object
use core::ops;
use super::thread::{ThreadPtr, RunState};
use super::s_runnable_threads;

/// An object on which a thread can sleep, woken by various event sources
///
/// This object should not be moved while references are active
pub struct SleepObject<'a>
{
	// Type that allows `fn get_ref` to borrow self and prevent moving
	_nomove: ::core::marker::PhantomData<&'a SleepObject<'a>>,
	name: &'static str,
	inner: crate::sync::Spinlock< SleepObjectInner >,
}
impl<'a> ::core::fmt::Debug for SleepObject<'a>
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		let lh = self.inner.lock();
		write!(f, "SleepObject(\"{}\" {} refs, flag={})", self.name, lh.reference_count, lh.flag)
	}
}
#[derive(Default)]
struct SleepObjectInner
{
	flag: bool,
	reference_count: usize,
	thread: Option<ThreadPtr>,
}

/// Referece to an active sleep object
pub struct SleepObjectRef
{
	// 'static is useful to avoid needing a lifetime param here... AND it prevents calling
	// get_ref again
	obj: *const SleepObject<'static>,
}
unsafe impl ::core::marker::Send for SleepObjectRef {}

impl<'a> SleepObject<'a>
{
	/// Create a new sleep object
	/// UNSAFE: The caller must ensure that this type's destructor is called (maintaining the correctness of obtained SleepObjectRef instances)
	pub const unsafe fn new(name: &'static str) -> SleepObject
	{
		SleepObject {
			_nomove: ::core::marker::PhantomData,
			name: name,
			inner: crate::sync::Spinlock::new(SleepObjectInner {
				flag: false,
				reference_count: 0,
				thread: None,
				}),
		}
	}
	/// Create a new sleep object and call a closure with it
	pub fn with_new<T>(name: &'static str, f: impl FnOnce(&mut SleepObject)->T) -> T {
		// SAFE: Destructor is called
		unsafe {
			let mut v = Self::new(name);
			// TODO: Pass a handle instead?
			f(&mut v)
		}
	}
	
	/// Wait the current thread on this object
	pub fn wait(&self)
	{
		//log_trace!("SleepObject::wait {:p} '{}'", self, self.name);
		
		let irql = crate::sync::hold_interrupts();
		let mut lh = self.inner.lock();
		assert!( lh.thread.is_none(), "A thread is already sleeping on object {:p} '{}'", self, self.name );
		
		if lh.flag == false
		{
			let mut cur = super::get_cur_thread();
			cur.run_state = RunState::Sleep(self as *const _ as *const () as *const _);	// Go via () to erase the lifetime
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
	//#[is_safe(irq)]	// Holds interrupts before locking
	pub fn signal(&self)
	{
		//log_trace!("SleepObject::signal {:p} '{}'", self, self.name);
		
		let _irq_lock = crate::sync::hold_interrupts();
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
	/// NOTE: After this is called, self must not move. This is enforced using a self-borrow
	pub fn get_ref(&'a self) -> SleepObjectRef {
		self.inner.lock().reference_count += 1;
		SleepObjectRef {
			obj: self as *const _ as *const () as *const _,
		}
	}
}

impl<'a> ops::Drop for SleepObject<'a>
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
		self.obj == obj as *const _ as *const () as *const SleepObject<'static>
	}
}
impl ops::Deref for SleepObjectRef
{
	type Target = SleepObject<'static>;
	
	fn deref(&self) -> &SleepObject<'static> {
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

