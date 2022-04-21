// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/metadevs/video/mod.rs
///! Video (Display) management
use crate::metadevs::video::geom::Pos;

/// Handle used by the display client (GUI) to control a mouse cursor
pub struct CursorHandle
{
	// Global cursor index
	//index: usize,
	
	// Visibility (true if the cursor is rendered)
	visible: bool,
	// Position on the virtual screen
	global_pos: Pos,
}

impl CursorHandle
{
	/// Construct a new cursor handle
	/// 
	/// NOTE: If two clients maintain a handle to the same cursor, they'll flight and the user will be confused
	pub const fn new() -> CursorHandle {
		CursorHandle {
			//index: 0,
			visible: true,
			global_pos: Pos::new(0,0),
			}
	}

	/// Obtain the current position of the cursor
	pub fn get_pos(&self) -> Pos {
		self.global_pos
	}
	/// Obtain the current visbiliy of the cursor
	pub fn is_visible(&self) -> bool {
		self.visible
	}

	/// Update the cursor position
	/// 
	/// NOTE: Will clip the position to within the bounds of the visible display area
	pub fn set_pos(&mut self, pos: Pos) {
		// TODO: Avoid clearing when surface doesn't change
		//if self.visible {
		//	super::with_display_at_pos( self.global_pos, |surf| surf.fb.move_cursor(None) );
		//}
		self.global_pos = super::get_closest_visible_pos(pos);
		if self.visible {
			super::with_display_at_pos( self.global_pos, |surf| {
				let pos = self.global_pos - surf.region.pos();
				surf.fb.move_cursor(Some(pos))
				});
		}
	}
	/// Show/hide the cursor
	pub fn set_visible(&mut self, visible: bool) {
		todo!("CursorHandle::set_visible - visible={}", visible);
	}
}
