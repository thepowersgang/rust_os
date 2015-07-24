// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/gui.rs
/// GUI syscall interface
use prelude::*;

use memory::freeze::Freeze;
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
		values::GUI_WIN_SHOWHIDE => todo!("GUI_WIN_SHOWHIDE"),
		values::GUI_WIN_REDRAW => todo!("GUI_WIN_REDRAW"),
		values::GUI_WIN_BLITRECT => {
			let x = try!( <u32>::get_arg(&mut args) );
			let y = try!( <u32>::get_arg(&mut args) );
			let w = try!( <u32>::get_arg(&mut args) );
			let h = try!( <u32>::get_arg(&mut args) );
			let data = try!( <Freeze<[u32]>>::get_arg(&mut args) );
			self.0.lock().blit_rect(Rect::new(x,y,w,h), &data);
			Ok(0)
			},
		_ => todo!("Window::handle_syscall({}, ...)", call),
		}
	}
    fn bind_wait(&self, flags: u32, obj: &mut ::threads::SleepObject) -> u32 {
        if flags & values::EV_GUI_WIN_INPUT != 0 {
            todo!("Window::bind_wait - input on obj={:?}", obj);
        }
        0
    }
    fn clear_wait(&self, flags: u32, obj: &mut ::threads::SleepObject) -> u32 {
        todo!("Window::clear_wait(flags={}, obj={:?})", flags, obj);
    }
}

#[derive(Default)]
struct PLWindowGroup( Option<::sync::Mutex< ::gui::WindowGroupHandle >> );
impl PLWindowGroup {
	fn with<O, F: FnOnce(&mut ::gui::WindowGroupHandle)->O>(&self, f: F) -> Result<O,u32> {
		match self.0
		{
		Some(ref v) => Ok( f(&mut v.lock()) ),
		None => Err(0),
		}
	}
}

pub fn newwindow(name: &str) -> Result<ObjectHandle,u32> {
	log_trace!("syscall_gui_newwindow(name={})", name);
	// Get window group for this process
	let wgh = ::threads::get_process_local::<PLWindowGroup>();
	wgh.with( |wgh| objects::new_object( Window(::sync::Mutex::new(wgh.create_window(name))) ) )
}

