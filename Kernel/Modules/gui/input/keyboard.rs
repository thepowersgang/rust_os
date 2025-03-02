// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/input/keyboard.rs
//! GUI Keyboard Arbitration

#[derive(Default,Debug)]
pub struct Instance(usize);

impl Instance
{
	pub fn new() -> Instance {
		Instance(0)
	}
	
	pub fn press_key(&self, key: KeyCode) {
		super::get_channel_by_index(self.0).handle_key(key, false);
	}
	pub fn release_key(&self, key: KeyCode) {
		super::get_channel_by_index(self.0).handle_key(key, true);
	}
}

pub use ::key_codes::KeyCode;

