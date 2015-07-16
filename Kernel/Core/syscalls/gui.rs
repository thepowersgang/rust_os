// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/gui.rs
/// GUI syscall interface
use prelude::*;

use super::{values,objects};
use super::{Error,ObjectHandle};
use super::SyscallArg;
use gui::{Rect};

pub fn newgroup(name: &str) -> Result<ObjectHandle,u32> {
	todo!("syscall_gui_newgroup(name={})", name);
}

struct Window(::sync::Mutex<::gui::WindowHandle>);
impl objects::Object for Window
{
	fn handle_syscall(&self, call: u16, mut args: &[usize]) -> Result<u64,Error>
	{
		match call
		{
		values::GUI_WIN_BLITRECT => {
			let x = try!( <u32>::get_arg(&mut args) );
			let y = try!( <u32>::get_arg(&mut args) );
			let w = try!( <u32>::get_arg(&mut args) );
			let h = try!( <u32>::get_arg(&mut args) );
			let data = try!( <&[u32]>::get_arg(&mut args) );
			self.0.lock().blit_rect(Rect::new(x,y,w,h), data);
			Ok(0)
			},
		_ => todo!("Window::handle_syscall({}, ...)", call),
		}
	}
}

pub fn newwindow(name: &str) -> Result<ObjectHandle,u32> {
	todo!("syscall_gui_newwindow(name={})", name);
}

