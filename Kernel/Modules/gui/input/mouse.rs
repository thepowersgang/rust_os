// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/input/mouse.rs
//! GUI Mouse Interface

#[derive(Default,Debug)]
pub struct Instance(usize);

impl Instance
{
	pub fn new() -> Instance {
		Instance(1)
	}
	
	pub fn move_cursor(&self, dx: i16, dy: i16) {
		super::get_channel_by_index(0).handle_mouse_move(dx, dy);
	}
	pub fn press_button(&self, btn: u8) {
		super::get_channel_by_index(0).handle_mouse_btn(btn, false);
	}
	pub fn release_button(&self, btn: u8) {
		super::get_channel_by_index(0).handle_mouse_btn(btn, true);
	}
}

