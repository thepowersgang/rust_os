// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/mod.rs
//! Kernel-mode side of the GUI
//!
//! Provides input routing and window management (i.e. exposing buffers to userland)
//!
//! Design Notes
//! ===
//! - Group windows into "screens" based on the owning session
//! - When a screen is hidden, a signalling message is sent to the controlling program (similar to the session leader in POSIX)
//!  - This allows the leader to switch to a lock screen
//! - All windows are backed by a framebuffer in this code
//!  - Kernel log is provided by a builtin text renderer
#![feature(linkage)]
#![no_std]

//#![feature(plugin)]
//#![plugin(tag_safe)]

#[macro_use]
extern crate kernel;

module_define!{GUI, [Video], init}

/// Initialise the GUI
fn init()
{
	// - Enumerate display devices
	::kernel::metadevs::video::register_geom_update(display_geom_update);
	input::init();
	// - Create kernel logging screen+window
	windows::init();
	kernel_log::init();
}

fn display_geom_update(new_total: ::kernel::metadevs::video::Rect)
{
	log_trace!("display_geom_update(new_total={})", new_total);
	
	windows::update_dims();
}

/// General window handling code
mod windows;
/// Kernel log display
mod kernel_log;

pub mod input;

// Import geometry types from video layer
pub use kernel::metadevs::video::{Dims, Pos, Rect};

pub use self::windows::WindowHandle;
pub use self::windows::WindowGroupHandle;

/// Pixel colour
#[derive(Copy,Clone)]
pub struct Colour(u32);

impl_fmt!{
	Debug(self, f) for Colour { write!(f, "Colour({:06x})", self.0) }
}
impl Colour
{
	pub fn def_black() -> Colour { Colour(0x00_00_00) }
	pub fn def_white() -> Colour { Colour(0xFF_FF_FF) }
	
	pub fn def_yellow() -> Colour { Colour(0xFF_FF_00) }
	pub fn def_green() -> Colour { Colour(0x00_FF_00) }
	
	pub fn as_argb32(&self) -> u32 { self.0 }
	pub fn from_argb32(v: u32) -> Self { Colour(v) }
}

