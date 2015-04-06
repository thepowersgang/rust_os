use _common::*;
use super::{Dims,Pos,Rect,Colour};
use sync::mutex::LazyMutex;

/// Window groups combine windows into "sessions", that can be switched with magic key combinations
struct WindowGroup
{
	/// Window group name, may be shown to the user if requested
	name: String,
	/// Canonical list of windows (sparse, for reallocation of IDs)
	windows: Vec<Option<Window>>,
	/// Render order (indexes into `windows`)
	render_order: Vec<usize>,
}

/// A single window, an arbitarily movable on-screen region
struct Window
{
	/// Position relative to the top-left of the display
	position: Pos,
	
	/// Window dimensions
	dims: Dims,
	/// Window backing buffer
	///
	/// TODO: This should be abstracted away such that the backing can be a on-card buffer
	buffer: Vec<u32>,
	
	
	/// Window title (queried by the decorator)
	title: String,
}

pub struct WindowGroupHandle(usize);
pub struct WindowHandle(usize,usize);

// TODO: When associated statics are implemented, replace this with a non-lazy mutex.
// - 13 sessions, #0 is fixed to be the kernel's log 1-12 are bound to F1-F12
static S_WINDOW_GROUPS: LazyMutex<Vec<Option<WindowGroup>>> = lazymutex_init!();


impl WindowGroupHandle
{
	pub fn alloc(name: &str) -> WindowGroupHandle {
		// Locate unused slot
		// - Return new in unused slot
		// if none, check against system session limit
		// - fail if too many
		// expand and return
		unimplemented!();
	}
	
	pub fn create_window(&mut self) -> WindowHandle {
		// Allocate a new window from the list
		unimplemented!();
	}
}
impl ::core::ops::Drop for WindowGroupHandle
{
	fn drop(&mut self)
	{
		unimplemented!();
	}
}

impl WindowHandle
{
	/// Redraw the window (mark for re-blitting)
	pub fn redraw(&mut self)
	{
		unimplemented!();
	}
	
	/// Fill an area of the window with a specific colour
	pub fn fill_rect(&mut self, area: Rect, colour: Colour)
	{
		log_trace!("(area={:?},colour={:?})", area, colour);
		unimplemented!();
	}
	
	/// Set single pixel (VERY inefficient, don't use unless you need to)
	pub fn pset(&mut self, pos: Pos, colour: Colour)
	{
		log_trace!("(pos={:?},colour={:?})", pos, colour);
		//self.scanline(pos.row())[pos.col()] = colour;
	}
}

impl ::core::ops::Drop for WindowHandle
{
	fn drop(&mut self)
	{
		unimplemented!();
	}
}

