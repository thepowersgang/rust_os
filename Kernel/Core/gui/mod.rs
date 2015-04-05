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
- Two core window types:
 - Graphical (a framebuffer)
 - Textual (fixed-width text, basic printing API)
*/
module_define!{GUI, [Video], init}

/// Initialise the GUI
fn init()
{
	// - Enumerate display devices
	//::metadevs::video::register_enumerate( enum_displays );
	// - Create kernel logging screen+window
	//let klog_session = windows::WindowGroup::alloc("Kernel");
	//s_kernel_log_window = text_window::TextWindow::new( klog_session.create_window() );
	// - Populate kernel logging window with accumulated logs
	// - Register to recieve logs
}

/// Abstracts the possibility of multiple output devices
mod multidisplay;
/// General window handling code
mod windows;
/// Handling for "text" windows
mod text_window;

/// Dimensions : Width/Height
struct Dims(u32,u32);	// W, H
/// Position : X/Y
struct Pos(i32,i32);
/// A generic rectangle
struct Rect(Pos,Dims);


