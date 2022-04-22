// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/event.rs
//! Asynchronous event waiter
#[allow(unused_imports)]
use crate::prelude::*;
use core::sync::atomic::{AtomicBool,Ordering};
use core::fmt;

/// A general-purpose wait event (when flag is set, waiters will be informed)
///
/// Only a single object can wait on this event at one time
///
/// TODO: Determine the set/reset conditions on the wait flag.
#[derive(Default)]
pub struct Source
{
	flag: AtomicBool,
	waiter: crate::sync::mutex::Mutex<Option<crate::threads::SleepObjectRef>>
}

/// An event structure that allows multiple waiters
pub struct ManySource
{
	flag: AtomicBool,
	waiters: super::Queue,
}

/// Event waiter
pub struct Waiter<'a>
{
	/// Event source
	source: Option<&'a Source>,
}

//static S_EVENT_NONE: Source = Source { flag: AtomicBool::new(false), waiter: mutex_init!(None) };

impl Source
{
	/// Create a new event source
	pub const fn new() -> Source
	{
		Source {
			flag: AtomicBool::new(false),
			waiter: crate::sync::mutex::Mutex::new(None),
		}
	}
	/// Return a wait handle for this event source
	pub fn wait<'a>(&'a self) -> Waiter<'a>
	{
		Waiter {
			source: Some(self),
			}
	}
	/// Raise the event (waking any attached waiter)
	pub fn trigger(&self)
	{
		//log_debug!("Trigger");
		self.flag.store(true, Ordering::SeqCst);	// prevents reodering around this
		self.waiter.lock().as_mut().map(|r| r.signal());
	}


	/// Register to wake the specified sleep object
	pub fn wait_upon(&self, waiter: &mut crate::threads::SleepObject) -> bool {
		{
			let mut lh = self.waiter.lock();
			assert!(lh.is_none());
			*lh = Some(waiter.get_ref());
		}
		self.flag.load(Ordering::SeqCst)	// Release - Don't reorder anything to after this
	}
	pub fn clear_wait(&self, _waiter: &mut crate::threads::SleepObject) {
		let mut lh = self.waiter.lock();
		*lh = None;
	}
}

impl ManySource
{
	pub const fn new() -> ManySource {
		ManySource {
			flag: AtomicBool::new(false),
			waiters: super::Queue::new(),
		}
	}

	/// Register to wake the specified sleep object
	pub fn wait_upon(&self, waiter: &mut crate::threads::SleepObject) -> bool {
		self.waiters.wait_upon(waiter);
		if self.flag.load(Ordering::SeqCst) {	// Release - Don't reorder anything to after this
			waiter.signal();
			true
		}
		else {
			false
		}
	}
	pub fn clear_wait(&self, waiter: &mut crate::threads::SleepObject) {
		self.waiters.clear_wait(waiter)
	}
}

impl<'a> fmt::Debug for Waiter<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "event::Waiter")
	}
}

impl<'a> super::PrimitiveWaiter for Waiter<'a>
{
	fn is_complete(&self) -> bool {
		self.source.is_none()
	}
	fn poll(&self) -> bool {
		match self.source {
		Some(r) => r.flag.swap(false, Ordering::Relaxed),
		None => true,
		}
	}
	fn run_completion(&mut self) {
		// Clear the source to mark this waiter as completed
		self.source = None;
	}
	fn bind_signal(&mut self, sleeper: &mut crate::threads::SleepObject) -> bool {
		if let Some(r) = self.source
		{
			// Store the sleep object reference
			*r.waiter.lock() = Some( sleeper.get_ref() );
			
			// If the waiter's flag is already set, return 'false' to force polling
			! r.flag.load(::core::sync::atomic::Ordering::Relaxed)
		}
		else
		{
			// Completed, don't impede sleeping
			true
		}
	}
	fn unbind_signal(&mut self) {
		if let Some(r) = self.source {
			*r.waiter.lock() = None;
		}
	}
}
