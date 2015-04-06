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
module_define!{GUI, [Video], init}

/// Initialise the GUI
fn init()
{
	// - Enumerate display devices
	//::metadevs::video::register_enumerate( enum_displays );
	// - Create kernel logging screen+window
	kernel_log::init();
}

/// Abstracts the possibility of multiple output devices
mod multidisplay;
/// General window handling code
mod windows;
/// Kernel log display
mod kernel_log;

/// Dimensions : Width/Height
#[derive(Copy,Clone,Debug)]
struct Dims(u32,u32);	// W, H
/// Position : X/Y
#[derive(Copy,Clone,Debug)]
struct Pos(i32,i32);
/// A generic rectangle
#[derive(Copy,Clone,Debug)]
struct Rect(Pos,Dims);
/// Pixel colour
#[derive(Copy,Clone,Debug)]
struct Colour(u32);


impl Colour
{
	pub fn def_black() -> Colour { Colour(0x00_00_00) }
	pub fn def_white() -> Colour { Colour(0xFF_FF_FF) }
	
	pub fn def_yellow() -> Colour { Colour(0xFF_FF_00) }
}

