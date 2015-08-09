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

pub trait Element
{
	fn render(&self, surface: ::surface::SurfaceView);
}

pub use window::Window;
pub use layout::{Frame,Box};
pub use input::text_box::TextInput;

