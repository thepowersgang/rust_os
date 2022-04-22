// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/queue.rs
//! Asynchronous wait queue
//!
//! Only wakes waiters that are currently waiting on the queue.
#[allow(dead_code)]
use core::fmt;

pub struct Waiter<'a>(Option<&'a Queue>);

/// A wait queue
///
/// Allows a list of threads to wait on a single object (e.g. a Mutex)
#[derive(Default)]
pub struct Queue
{
	// TODO: Have a local SleepObjectRef to avoid malloc on single-wait case
	waiters: crate::sync::mutex::Mutex< crate::lib::Queue<crate::threads::SleepObjectRef> >,
}


impl Queue
{
	/// Create a new queue source
	pub const fn new() -> Queue
	{
		Queue {
			waiters: crate::sync::mutex::Mutex::new(crate::lib::Queue::new()),
		}
	}
	
	/// Create a waiter for this queue
	///
	/// The passed handler is called with None to poll the state.
	// TODO: Race conditions between 'Source::wait_on' and 'wait_on_list'.
	pub fn wait_on<'a>(&'a self) -> Waiter
	{
		// TODO: Requires a queue wait variant
		unimplemented!();
	}

	pub fn wait_upon(&self, waiter: &mut crate::threads::SleepObject) {
		let mut wh = self.waiters.lock();
		wh.push( waiter.get_ref() );
	}
	pub fn clear_wait(&self, waiter: &mut crate::threads::SleepObject) {
		self.waiters.lock().filter_out(|ent| ent.is_from(waiter));
	}
	
	/// Wake a single waiting thread
	pub fn wake_one(&self) -> bool
	{
		let mut lh = self.waiters.lock();
		if let Some(waiter) = lh.pop()
		{
			waiter.signal();
			true
		}
		else
		{
			false
		}
	}

	/// Wake all waiting threads
	pub fn wake_all(&self)
	{
		let mut lh = self.waiters.lock();
		while let Some(waiter) = lh.pop() {
			waiter.signal();
		}
	}
}

impl<'a> fmt::Debug for Waiter<'a>
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "queue::Waiter")
	}
}
impl<'a> super::PrimitiveWaiter for Waiter<'a>
{
	fn is_complete(&self) -> bool {
		self.0.is_none()
	}
	fn poll(&self) -> bool {
		unimplemented!();
	}
	fn run_completion(&mut self) {
		unimplemented!();
	}
	fn bind_signal(&mut self, _sleeper: &mut crate::threads::SleepObject) -> bool {
		unimplemented!();
	}
	fn unbind_signal(&mut self) {
		unimplemented!();
	}
}

