// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/timer.rs
//! Asynchronous Timer.
//! 
//! An async timer type, firing after the specified duration has elapsed
//!
//! TODO: Fix to actually be usable

pub struct Timer
{
	expiry_ticks: u64,
}

impl Timer
{
	//pub fn new(duration_ms: u64) -> Timer
	pub fn new(duration_ms: u64) -> super::Waiter<'static>
	{
		unimplemented!()
	}
}

