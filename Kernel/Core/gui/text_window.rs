// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/text_window.rs
// - Rendering for text-only windows (i.e. the kernel log)
use _common::*;
use super::windows::WindowHandle;

struct TextWindow(WindowHandle);

struct TextColour(u32,u32);

impl TextWindow
{
	fn new(wh: WindowHandle) -> TextWindow {
		TextWindow(wh)
	}
	
	/// Render a text string at `pos` using `colour`
	///
	/// Returns `Ok(n_slots)` when the string was fully rendered, and `Err(n_slots)` when it was truncated
	pub fn render(pos: super::Pos, colour: TextColour, string: &str) -> Result<u32,u32>
	{
		unimplemented!()
	}
}

