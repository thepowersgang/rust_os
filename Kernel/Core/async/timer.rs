// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/timer.rs
//! Asynchronous Timer.
//! 
//! An async timer type, firing after the specified duration has elapsed
//!
//! TODO: Fix to actually be usable

pub struct Waiter
{
	expiry_ticks: u64,
}

impl Waiter
{
	pub fn new(duration_ms: u64) -> Waiter
	{
		Waiter {
			expiry_ticks: crate::time::ticks() + duration_ms,
		}
	}
}

impl super::PrimitiveWaiter for Waiter {
	fn is_complete(&self) -> bool {
		crate::time::ticks() >= self.expiry_ticks
	}
	
	fn poll(&self) -> bool {
		self.is_complete()
	}
	fn run_completion(&mut self) {
		// no action
	}
	fn bind_signal(&mut self, _sleeper: &mut crate::threads::SleepObject) -> bool {
		todo!("timer::Waiter::bind_signal()")
		//::time::bind_signal(_sleeper, self.expiry_ticks)
	}
	fn unbind_signal(&mut self) {
		todo!("timer::Waiter::unbind_signal()")
	}
}

impl ::core::fmt::Debug for Waiter {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		write!(f, "timer::Waiter({})", self.expiry_ticks)
	}
}

