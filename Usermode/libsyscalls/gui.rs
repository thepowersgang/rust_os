// Tifflin OS - System Calls
// - By John Hodge (thePowersGang)
//
// gui.rs
use core::prelude::*;

pub use ::values::GuiEvent as Event;

pub struct Group(super::ObjectHandle);
pub struct Window(super::ObjectHandle);

pub	struct Rect { pub p: Pos, pub d: Dims, }
impl Rect {
	pub fn new(x: u32, y: u32, w: u32, h: u32) -> Rect {
		Rect { p: Pos { x:x, y:y }, d: Dims { w:w, h:h } }
	}
}
pub struct Pos { pub x: u32, pub y: u32, }
pub struct Dims { pub w: u32, pub h: u32, }

#[derive(Copy,Clone)]
pub struct Colour(u32);
impl Colour {
	pub fn as_argb32(&self) -> u32 { self.0 }
	pub fn white() -> Colour { Colour(0xFFFFFF) }
}



include!("../../keycodes.inc.rs");


impl Group
{
	pub fn new(name: &str) -> Result<Group,()>
	{
		match super::ObjectHandle::new( unsafe { syscall!(GUI_NEWGROUP, name.as_ptr() as usize, name.len()) } as usize )
		{
		Ok(rv) => Ok( Group(rv) ),
		Err(code) => {
			panic!("TODO: Error code {}", code);
			},
		}
	}
	
	pub fn force_active(&self) -> Result<(),()> {
		match super::to_result( unsafe { self.0.call_0(::values::GUI_GRP_FORCEACTIVE) } as usize )
		{
		Ok(_) => Ok( () ),
		Err(_) => Err( () ),
		}
	}
}
impl ::Object for Group
{
	const CLASS: u16 = ::values::CLASS_GUI_GROUP;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: super::ObjectHandle) -> Self {
		Group(handle)
	}
	fn into_handle(self) -> ::ObjectHandle { self.0 }
	fn get_wait(&self) -> ::values::WaitItem {
		self.0.get_wait( ::values::EV_GUI_GRP_SHOWHIDE )
	}
	fn check_wait(&self, wi: &::values::WaitItem) {
		assert_eq!(wi.object, self.0 .0);
		if wi.flags & ::values::EV_GUI_GRP_SHOWHIDE != 0 {
			// TODO
		}
	}
}

pub fn set_group(grp: Group)
{
	use Object;
	unsafe { syscall!(GUI_BINDGROUP, grp.into_handle().into_raw() as usize); }
}

impl Window
{
	pub fn new(name: &str) -> Result<Window,()>
	{
		match super::ObjectHandle::new( unsafe { syscall!(GUI_NEWWINDOW, name.as_ptr() as usize, name.len()) } as usize )
		{
		Ok(rv) => Ok( Window(rv) ),
		Err(code) => {
			panic!("TODO: Error code {}", code);
			},
		}
	}
	
	pub fn show(&self) {
		unsafe { self.0.call_2(::values::GUI_WIN_SETFLAG, ::values::GUI_WIN_FLAG_VISIBLE as usize, 1); }
	}
	pub fn hide(&self) {
		unsafe { self.0.call_2(::values::GUI_WIN_SETFLAG, ::values::GUI_WIN_FLAG_VISIBLE as usize, 0); }
	}
	pub fn redraw(&self) {
		unsafe { self.0.call_0(::values::GUI_WIN_REDRAW); }
	}

	// TODO: Should this be controllable by the application?
	pub fn maximise(&self) {
		unsafe { self.0.call_2(::values::GUI_WIN_SETFLAG, ::values::GUI_WIN_FLAG_MAXIMISED as usize, 1); }
	}
	
	pub fn blit_rect(&self, x: u32, y: u32, w: u32, h: u32, data: &[u32]) {
		unsafe { self.0.call_6(::values::GUI_WIN_BLITRECT, x as usize, y as usize, w as usize, h as usize, data.as_ptr() as usize, data.len()); }
	}
	pub fn fill_rect(&self, x: u32, y: u32, w: u32, h: u32, colour: u32) {
		unsafe { self.0.call_5(::values::GUI_WIN_FILLRECT, x as usize, y as usize, w as usize, h as usize, colour as usize); }
	}

	pub fn pop_event(&self) -> Option<::values::GuiEvent> {
		let v = unsafe { self.0.call_0(::values::GUI_WIN_GETEVENT) };
		if v == !0 {
			None
		}
		else {
			Some( ::values::GuiEvent::from(v) )
		}
	}
}
impl ::Object for Window
{
	const CLASS: u16 = ::values::CLASS_GUI_WIN;
	fn class() -> u16 { Self::CLASS }
	fn from_handle(handle: super::ObjectHandle) -> Self {
		Window(handle)
	}
	fn into_handle(self) -> ::ObjectHandle { self.0 }
	
	fn get_wait(&self) -> ::values::WaitItem {
		self.0.get_wait( ::values::EV_GUI_WIN_INPUT )
	}
	fn check_wait(&self, wi: &::values::WaitItem) {
		assert_eq!(wi.object, self.0 .0);
		if wi.flags & ::values::EV_GUI_WIN_INPUT != 0 {
			// TODO
		}
	}
}

