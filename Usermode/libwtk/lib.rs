//
//
//
//! Tifflin window toolkit
#![feature(zero_one)]
#![feature(const_fn)]

extern crate async;
extern crate byteorder;

#[macro_use]
extern crate macros;

#[macro_use]
extern crate syscalls;

mod geom;
mod surface;

mod window;
mod layout;
mod static_layout;
mod input;
mod text;

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
	fn render(&self, surface: ::surface::SurfaceView, force: bool);

	fn element_at_pos(&self, x: u32, y: u32) -> (&::Element,(u32,u32));
}
/// Object safe
impl<'a, T: Element> Element for &'a T
{
	fn focus_change(&self, have: bool) { (*self).focus_change(have) }
	fn handle_event(&self, ev: ::InputEvent, win: &mut ::window::Window) -> bool { (*self).handle_event(ev, win) }
	fn render(&self, surface: ::surface::SurfaceView, force: bool) { (*self).render(surface, force) }
	fn element_at_pos(&self, x: u32, y: u32) -> (&::Element,(u32,u32)) { (*self).element_at_pos(x,y) }
}
/// Unit type is a valid element. Just does nothing.
impl Element for ()
{
	fn focus_change(&self, _have: bool) { }
	fn handle_event(&self, _ev: ::InputEvent, _win: &mut ::window::Window) -> bool { false }
	fn render(&self, _surface: ::surface::SurfaceView, _force: bool) { }
	fn element_at_pos(&self, _x: u32, _y: u32) -> (&::Element,(u32,u32)) { (self,(0,0)) }
}

pub use window::Window;
pub use layout::{Frame,Box};
pub use input::text_box::TextInput;
pub use input::button::Button;
pub use image::Image;

pub use static_layout::Box as StaticBox;
pub use static_layout::BoxEle;

pub use text::Label;

pub fn initialise()
{
	use syscalls::Object;
	use syscalls::threads::{S_THIS_PROCESS,ThisProcessWaits};
	::syscalls::threads::wait(&mut [S_THIS_PROCESS.get_wait(ThisProcessWaits::new().recv_obj())], !0);
	::syscalls::gui::set_group( S_THIS_PROCESS.receive_object::<::syscalls::gui::Group>(0).unwrap() );
}
