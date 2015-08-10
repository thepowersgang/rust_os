// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/input/keyboard.rs
//! GUI Keyboard Arbitration

#[derive(Default,Debug)]
pub struct Instance(usize);

struct KeyboardState
{
	//	caps: bool,	// Caps can DIAF
	shift: u8,
	ctrl: u8,
	alt: u8,
	altgr: bool,

	ime_ofs: u8,
	ime_val: u32,
}

impl Instance
{
	pub fn new() -> Instance {
		Instance(1)
	}
	
	pub fn press_key(&self, key: KeyCode) {
		super::get_channel_by_index(0).handle(super::Event::KeyDown(key));
	}
	pub fn release_key(&self, key: KeyCode) {
		super::get_channel_by_index(0).handle(super::Event::KeyUp(key));
	}
}

include!("../../../../keycodes.inc.rs");

