
pub use crate::surface::Colour;

/// Elements for controlling actions (buttons, scrollbars, tabs, ...)
pub mod controls;
/// Elements for displaying
pub mod display;
/// Elements for taking in user input (text boxes, dropdowns, radio/check-boxes)
pub mod input;
/// Dynamic element layout
pub mod dynamic_layout;
/// Static element layout
pub mod static_layout;

/// Visual seperators (frames, lines)
pub mod separators;

/// Solid colour
impl crate::Element for Colour
{
	fn resize(&self, _w: u32, _u: u32) {}
	fn render(&self, surface: ::surface::SurfaceView, force: bool) {
		if force {
			surface.fill_rect(crate::geom::Rect::new(0,0,!0,!0), *self);
		}
	}
	fn with_element_at_pos(&self, pos: ::geom::PxPos, _dims: ::geom::PxDims, f: ::WithEleAtPosCb) -> bool { f(self, pos) }
}