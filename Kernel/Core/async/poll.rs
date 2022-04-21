// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/poll.rs
//! Polling async waiter
use crate::prelude::*;
use core::cell::RefCell;
use core::fmt;

/// Callback for a poll waiter
// RefCell allows calling FnMut from poll(&self)
type PollCb<'a> = RefCell<Box<dyn for<'r> FnMut(Option<&'r mut Waiter<'a>>) -> bool + Send + 'a>>;

pub struct Waiter<'a>( Option<PollCb<'a>> );

impl<'a> Waiter<'a>
{
	pub fn null() -> Waiter<'a> {
		Waiter(None)
	}
	pub fn new<F>(f: F) -> Waiter<'a>
	where
		F: FnMut(Option<&mut Waiter<'a>>)->bool + Send + 'a
	{
		Waiter( Some(RefCell::new(box f)) )
	}
}

impl<'a> fmt::Debug for Waiter<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "poll::Waiter")
	}
}

impl<'a> super::PrimitiveWaiter for Waiter<'a>
{
	fn is_complete(&self) -> bool {
		match self.0
		{
		Some(_) => false,
		None => true,
		}
	}
	fn poll(&self) -> bool {
		match self.0
		{
		Some(ref cb) => {
			let mut b = cb.borrow_mut();
			let rb = &mut **b;
			// Call poll hander with 'None' to ask it to poll
			rb(None)
			},
		None => true,
		}
	}
	fn run_completion(&mut self) {
		match self.0.take()
		{
		// Pass 'Some(self)' to indicate completion 
		Some(cb) => {
			cb.into_inner()( Some(self) );
			},
		None => {
			},
		}
	}
	fn bind_signal(&mut self, _sleeper: &mut crate::threads::SleepObject) -> bool {
		false
	}
	fn unbind_signal(&mut self) {
	}
}

