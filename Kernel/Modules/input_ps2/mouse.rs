// "Tifflin" Kernel - ATA Driver
// - By John Hodge (thePowersGang)
//
// Modules/input_ps2/mouse.rs
//! PS2 Mouse driver
use kernel::prelude::*;

#[derive(Debug)]
pub enum Type
{
	Std,
	Scroll,
	QuintBtn,	// 5 buttons
}

#[derive(Debug)]
pub struct Dev;

impl Dev
{
	pub fn new(ty: Type) -> (Option<u8>,Dev) {
		log_warning!("TODO: PS2 Mouse driver {:?}", ty);
		(None, Dev)
	}
	
	pub fn recv_byte(&mut self, byte: u8) -> Option<u8> {
		log_warning!("TODO: PS/2 mouse input {:#02x}", byte);
		None
	}
}


