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

/// Re-export GUI events for users of the library
pub use syscalls::gui::Event as InputEvent;

pub trait Element
{
	fn focus_change(&self, _have: bool) {}
	fn render(&self, surface: ::surface::SurfaceView);
	fn handle_event(&self, ev: ::InputEvent, win: &mut ::window::Window) -> bool { false }
}

pub use window::Window;
pub use layout::{Frame,Box};
pub use input::text_box::TextInput;

