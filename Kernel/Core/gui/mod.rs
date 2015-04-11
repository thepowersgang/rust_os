// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/mod.rs
// - Kernel compositor core
//! Kernel-mode side of the GUI
//! Provides input routing and window management (i.e. exposing buffers to userland)
/*!
Design Notes
===
- Group windows into "screens" based on the owning session
- When a screen is hidden, a signalling message is sent to the controlling program (similar to the session leader in POSIX)
 - This allows the leader to switch to a lock screen
- All windows are backed by a framebuffer in this code
 - Kernel log is provided by a builtin text renderer
*/
use _common::*;
module_define!{GUI, [Video], init}

/// Initialise the GUI
fn init()
{
	// - Enumerate display devices
	::metadevs::video::register_geom_update(display_geom_update);
	// - Create kernel logging screen+window
	windows::init();
	kernel_log::init();
}

fn display_geom_update(new_total: ::metadevs::video::Rect)
{
	log_trace!("display_geom_update(new_total={})", new_total);
	//if !was_insertion {
	//	unimplemented!();
	//}
	//else {
	//	// Update 
	//	unimplemented!();
	//}
	
	windows::update_dims();
}

/// General window handling code
mod windows;
/// Kernel log display
mod kernel_log;

// Import geometry types from video layer
pub use metadevs::video::{Dims, Pos, Rect};

/// Pixel colour
#[derive(Copy,Clone)]
struct Colour(u32);

impl_fmt!{
	Debug(self, f) for Colour { write!(f, "Colour({:06x})", self.0) }
}
impl Colour
{
	pub fn def_black() -> Colour { Colour(0x00_00_00) }
	pub fn def_white() -> Colour { Colour(0xFF_FF_FF) }
	
	pub fn def_yellow() -> Colour { Colour(0xFF_FF_00) }
	
	pub fn as_argb32(&self) -> u32 { self.0 }
}

