//
//
//
//! Tifflin window toolkit
#![feature(zero_one)]
#![feature(const_fn)]
#![feature(core_slice_ext)]

extern crate async;

#[macro_use]
extern crate macros;

#[macro_use]
extern crate syscalls;

mod geom;
mod surface;

mod window;
mod layout;
mod input;

pub mod image;

pub use surface::Colour;

/// Re-export GUI events for users of the library
pub use syscalls::gui::Event as InputEvent;

/// Common trait for window elements
pub trait Element
{
	/// Called when focus changes to/from this element
	fn focus_change(&self, _have: bool) {}
	/// Called when an event fires. Keyboard events are controlled by focus, mouse via the render tree
	fn handle_event(&self, _ev: ::InputEvent, _win: &mut ::window::Window) -> bool { false }
	/// Redraw this element into the provided surface view
	fn render(&self, surface: ::surface::SurfaceView);
}
/// Object safe
impl<'a, T: Element> Element for &'a T
{
	fn focus_change(&self, have: bool) { (*self).focus_change(have) }
	fn handle_event(&self, ev: ::InputEvent, win: &mut ::window::Window) -> bool { (*self).handle_event(ev, win) }
	fn render(&self, surface: ::surface::SurfaceView) { (*self).render(surface) }
}
/// Unit type is a valid element. Just does nothing.
impl Element for ()
{
	fn focus_change(&self, _have: bool) { }
	fn handle_event(&self, _ev: ::InputEvent, _win: &mut ::window::Window) -> bool { false }
	fn render(&self, _surface: ::surface::SurfaceView) { }
}

pub use window::Window;
pub use layout::{Frame,Box};
pub use input::text_box::TextInput;
pub use input::button::Button;
pub use image::Image;

