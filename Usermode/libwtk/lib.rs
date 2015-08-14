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

mod image;

/// Re-export GUI events for users of the library
pub use syscalls::gui::Event as InputEvent;

pub trait Element
{
	fn focus_change(&self, _have: bool) {}
	fn render(&self, surface: ::surface::SurfaceView);
	fn handle_event(&self, ev: ::InputEvent, win: &mut ::window::Window) -> bool { false }
}
impl<'a, T: Element> Element for &'a T
{
	fn focus_change(&self, have: bool) { (*self).focus_change(have) }
	fn render(&self, surface: ::surface::SurfaceView) { (*self).render(surface) }
	fn handle_event(&self, ev: ::InputEvent, win: &mut ::window::Window) -> bool { (*self).handle_event(ev, win) }
}
impl Element for ()
{
	fn focus_change(&self, have: bool) { }
	fn render(&self, surface: ::surface::SurfaceView) { }
	fn handle_event(&self, ev: ::InputEvent, win: &mut ::window::Window) -> bool { false }
}

pub use window::Window;
pub use layout::{Frame,Box};
pub use input::text_box::TextInput;
pub use input::button::Button;
pub use image::Image;

