//
//
//
//! Tifflin window toolkit
#![feature(unboxed_closures,fn_traits)]

extern crate r#async;
extern crate byteorder;
extern crate embedded_images;

#[macro_use]
extern crate macros;

#[macro_use]
extern crate syscalls;

pub mod geom;
pub mod element_trait;
pub mod elements;

pub mod surface;
mod window;

pub mod menu;

pub mod decorator;

pub use crate::element_trait::Element;
pub use crate::surface::Colour;

/// Re-export GUI events for users of the library
pub use syscalls::gui::Event as InputEvent;
pub use syscalls::gui::KeyCode as KeyCode;
pub use window::Modifier as ModifierKey;
/// Re-export async to reduce downstream dependencies
pub use r#async::idle_loop;

pub type WithEleAtPosCb<'a> = &'a mut dyn FnMut(&dyn Element, ::geom::PxPos) -> bool;

pub use window::{Window, WindowTrait};

/// Initialise the WTK library with a window group handle sent by the parent process
pub fn initialise()
{
	use syscalls::threads::S_THIS_PROCESS;
	::syscalls::gui::set_group( S_THIS_PROCESS.receive_object("guigrp").unwrap() );
}

