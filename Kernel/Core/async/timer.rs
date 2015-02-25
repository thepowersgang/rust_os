// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/timer.rs
///! Asynchronous Timer

pub struct Timer
{
	expiry_ticks: u64,
}

impl Timer
{
	//pub fn new(duration_ms: u64) -> Timer
	pub fn new(duration_ms: u64) -> super::EventWait<'static>
	{
		unimplemented!()
	}
}

