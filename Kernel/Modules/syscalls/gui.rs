// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/gui.rs
/// GUI syscall interface
use kernel::prelude::*;

use kernel::memory::freeze::{Freeze,FreezeMut};
use gui::{Rect};
use kernel::sync::Mutex;

use super::{values,objects};
use super::{Error,ObjectHandle};
use super::SyscallArg;

impl ::core::convert::Into<values::GuiEvent> for ::gui::input::Event {
	fn into(self) -> values::GuiEvent {
		use gui::input::Event;
		match self
		{
		Event::KeyUp  (kc)  => values::GuiEvent::KeyUp  (From::from(kc as u8)),
		Event::KeyDown(kc)  => values::GuiEvent::KeyDown(From::from(kc as u8)),
		Event::Text   (buf) => values::GuiEvent::Text   (From::from(buf)),
		Event::MouseMove(x,y, dx,dy) => values::GuiEvent::MouseMove(x,y, dx,dy),
		Event::MouseUp(x,y,btn) => values::GuiEvent::MouseUp(y,x,btn),
		Event::MouseDown(x,y,btn) => values::GuiEvent::MouseDown(x,y,btn),
		}
	}
}


#[inline(never)]
pub fn newgroup(name: &str) -> Result<ObjectHandle,u32> {
	// Only init can create new sessions
	// TODO: Use a capability system instead of hardcoding to only PID0
	if ::kernel::threads::get_process_id() == 0 {
		Ok(objects::new_object(Group(::gui::WindowGroupHandle::alloc(name))))
	}
	else {
		todo!("syscall_gui_newgroup(name={}) - PID != 0", name);
	}
}

#[inline(never)]
pub fn bind_group(object_handle: u32) -> Result<bool,Error> {
	let wgh = ::kernel::threads::get_process_local::<PLWindowGroup>();
	let mut h = wgh.0.lock();
	if h.is_none() {
		let group: Group = try!(::objects::take_object(object_handle));
		*h = Some(group.0);
		Ok(true)
	}
	else {
		Ok(false)
	}
}

#[inline(never)]
pub fn get_group() -> Result<ObjectHandle,u32>
{
	let wgh = ::kernel::threads::get_process_local::<PLWindowGroup>();
	wgh.with(|h| objects::new_object(Group( h.clone() )))
}

/// Window group, aka Session
struct Group(::gui::WindowGroupHandle);
impl objects::Object for Group
{
	const CLASS: u16 = values::CLASS_GUI_GROUP;
	fn class(&self) -> u16 { Self::CLASS }
	fn as_any(&self) -> &Any { self }
	fn handle_syscall(&self, call: u16, _args: &[usize]) -> Result<u64,Error>
	{
		match call
		{
		values::GUI_GRP_FORCEACTIVE => {
			if ::kernel::threads::get_process_id() == 0 {
				self.0.force_active();
				Ok(0)
			}
			else {
				Ok(1)
			}
			},
		_ => todo!("Group::handle_syscall({}, ...)", call),
		}
	}
	fn bind_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
		if flags & values::EV_GUI_GRP_SHOWHIDE != 0 {
			todo!("Group::bind_wait - showhide on obj={:?}", obj);
		}
		0
	}
	fn clear_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
		todo!("Group::clear_wait(flags={}, obj={:?})", flags, obj);
	}
}

/// Window
struct Window(Mutex<::gui::WindowHandle>);
impl objects::Object for Window
{
	const CLASS: u16 = values::CLASS_GUI_WIN;
	fn class(&self) -> u16 { Self::CLASS }
	fn as_any(&self) -> &Any { self }
	fn handle_syscall(&self, call: u16, mut args: &[usize]) -> Result<u64,Error>
	{
		match call
		{
		values::GUI_WIN_SETFLAG => {
			let flag  = try!( <u8>::get_arg(&mut args) );
			let is_on = try!( <bool>::get_arg(&mut args) );
			match flag
			{
			values::GUI_WIN_FLAG_VISIBLE   => if is_on { self.0.lock().show()	 } else { self.0.lock().hide() },
			values::GUI_WIN_FLAG_MAXIMISED => if is_on { self.0.lock().maximise() } else { todo!("Unmaximise window"); },
			_ => todo!("Window::handle_syscall(GUI_WIN_SETFLAG, {} := {}) - Unknown flag", flag, is_on),
			}
			Ok(0)
			},
		values::GUI_WIN_REDRAW => {
			self.0.lock().redraw();
			Ok(0)
			},
		values::GUI_WIN_BLITRECT => {
			let x = try!( <u32>::get_arg(&mut args) );
			let y = try!( <u32>::get_arg(&mut args) );
			let w = try!( <u32>::get_arg(&mut args) );
			let data = try!( <Freeze<[u32]>>::get_arg(&mut args) );
			let stride = try!( <usize>::get_arg(&mut args) );
			if data.len() == 0 {
				Ok(0)
			}
			else {
				// data.len() should be (h-1)*stride + w long
				let h = if data.len() >= w as usize {
						((data.len() - w as usize) / stride) as u32 + 1
					} else {
						1
					};
				self.0.lock().blit_rect(Rect::new(x,y,w,h), &data, stride);
				Ok(0)
			}
			},
		values::GUI_WIN_FILLRECT => {
			let x = try!( <u32>::get_arg(&mut args) );
			let y = try!( <u32>::get_arg(&mut args) );
			let w = try!( <u32>::get_arg(&mut args) );
			let h = try!( <u32>::get_arg(&mut args) );
			let colour = try!( <u32>::get_arg(&mut args) );
			self.0.lock().fill_rect(Rect::new(x,y,w,h), ::gui::Colour::from_argb32(colour));
			Ok(0)
			},
		values::GUI_WIN_GETEVENT => {
			match self.0.lock().pop_event()
			{
			Some(ev) => {
				let mut ev_ptr = try!( <FreezeMut<values::GuiEvent>>::get_arg(&mut args) );
				*ev_ptr = ev.into();
				Ok(0)
				},
			None => Ok(!0),
			}
			},
		values::GUI_WIN_GETDIMS => {
			let d = self.0.lock().get_dims();
			let rv = (d.w as u64) << 32 | (d.h as u64);
			Ok( rv )
			},
		values::GUI_WIN_SETDIMS => {
			let w = try!( <u32>::get_arg(&mut args) );
			let h = try!( <u32>::get_arg(&mut args) );
			let d = {
				let mut lh = self.0.lock();
				lh.resize( ::gui::Dims::new(w, h) );
				lh.get_dims()
				};
			let rv = (d.w as u64) << 32 | (d.h as u64);
			Ok( rv )
			},
		values::GUI_WIN_GETPOS => {
			let p = self.0.lock().get_pos();
			let rv = (p.x as u64) << 32 | (p.y as u64);
			Ok( rv )
			},
		values::GUI_WIN_SETPOS => {
			let x = try!( <u32>::get_arg(&mut args) );
			let y = try!( <u32>::get_arg(&mut args) );
			let p = {
				let mut lh = self.0.lock();
				lh.set_pos( ::gui::Pos::new(x, y) );
				lh.get_pos()
				};
			let rv = (p.x as u64) << 32 | (p.y as u64);
			Ok( rv )
			},
		values::GUI_WIN_SETCLIENTREGION => {
			let x = try!( <u32>::get_arg(&mut args) );
			let y = try!( <u32>::get_arg(&mut args) );
			let w = try!( <u32>::get_arg(&mut args) );
			let h = try!( <u32>::get_arg(&mut args) );
			self.0.lock().set_client_region(Rect::new(x,y, w,h));
			Ok(0)
			},
		_ => {
			log_error!("TODO: Window::handle_syscall({}, ...)", call);
			Err(Error::UnknownCall)
			},
		}
	}
	fn bind_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
		let mut ret = 0;
		if flags & values::EV_GUI_WIN_INPUT != 0 {
			self.0.lock().wait_input(obj);
			ret |= values::EV_GUI_WIN_INPUT;
		}
		ret
	}
	fn clear_wait(&self, flags: u32, obj: &mut ::kernel::threads::SleepObject) -> u32 {
		if flags & values::EV_GUI_WIN_INPUT != 0 {
			self.0.lock().clear_wait_input(obj);
		}
		0
	}
}

#[derive(Default)]
struct PLWindowGroup( Mutex<Option< ::gui::WindowGroupHandle >> );
impl PLWindowGroup {
	fn with<O, F: FnOnce(&mut ::gui::WindowGroupHandle)->O>(&self, f: F) -> Result<O,u32> {
		match *self.0.lock()
		{
		Some(ref mut v) => Ok( f(v) ),
		None => Err(0),
		}
	}
}

#[inline(never)]
pub fn newwindow(name: &str) -> Result<ObjectHandle,u32> {
	log_trace!("syscall_gui_newwindow(name={})", name);
	// Get window group for this process
	let wgh = ::kernel::threads::get_process_local::<PLWindowGroup>();
	wgh.with( |wgh| objects::new_object( Window(Mutex::new(wgh.create_window(name))) ) )
}

