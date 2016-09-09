// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/queue.rs
//! Asynchronous wait queue
#[allow(dead_code)]
use core::fmt;

pub struct Waiter<'a>(Option<&'a Source>);

/// A wait queue
///
/// Allows a list of threads to wait on a single object (e.g. a Mutex)
#[derive(Default)]
pub struct Source
{
	// TODO: Have a local SleepObjectRef to avoid malloc on single-wait case
	waiters: ::sync::mutex::Mutex< ::lib::Queue<::threads::SleepObjectRef> >,
}


impl Source
{
	/// Create a new queue source
	pub const fn new() -> Source
	{
		Source {
			waiters: ::sync::mutex::Mutex::new(::lib::Queue::new()),
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

	pub fn wait_upon(&self, waiter: &mut ::threads::SleepObject) {
		let mut wh = self.waiters.lock();
		wh.push( waiter.get_ref() );
	}
	pub fn clear_wait(&self, waiter: &mut ::threads::SleepObject) {
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
	fn bind_signal(&mut self, _sleeper: &mut ::threads::SleepObject) -> bool {
		unimplemented!();
	}
	fn unbind_signal(&mut self) {
		unimplemented!();
	}
}

