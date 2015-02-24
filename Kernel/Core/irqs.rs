// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/irqs.rs
//! Core IRQ Abstraction
use _common::*;

pub struct Handle
{
	index: u32,
	// TODO: Have a Rc'd (or boxed) async event source
	event: Box<::async::EventSource>,
}

// Needs to hand off (somehow) to the architecture's IRQ support
pub fn bind_interrupt_event(num: u32) -> Handle
{
	// 1. (if not already) bind a handler on the architecture's handlers
	// 2. Add this handler to the meta-handler
	// 3. Enable this vector on the architecture
	unimplemented!()
}

impl Handle
{
	pub fn get_event(&self) -> &::async::EventSource
	{
		&*self.event
	}
}

