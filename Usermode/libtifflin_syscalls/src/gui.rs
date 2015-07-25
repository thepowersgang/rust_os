// Tifflin OS - System Calls
// - By John Hodge (thePowersGang)
//
// gui.rs
use core::prelude::*;

pub struct Group(super::ObjectHandle);
pub struct Window(super::ObjectHandle);


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
}
impl ::Object for Group
{
	const CLASS: u16 = ::values::CLASS_GUI_GROUP;
	fn from_handle(handle: super::ObjectHandle) -> Self {
		Group(handle)
	}
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
		unsafe { self.0.call_1(::values::GUI_WIN_SHOWHIDE, 1); }
	}
	pub fn hide(&self) {
		unsafe { self.0.call_1(::values::GUI_WIN_SHOWHIDE, 0); }
	}
	pub fn redraw(&self) {
		unsafe { self.0.call_0(::values::GUI_WIN_REDRAW); }
	}

	// TODO: Should this be controllable by the application?
	pub fn maximise(&self) {
		//todo!("Window::maximise");
	}
	
	pub fn blitrect(&self, x: u32, y: u32, w: u32, h: u32, data: &[u32]) {
		unsafe { self.0.call_6(::values::GUI_WIN_BLITRECT, x as usize, y as usize, w as usize, h as usize, data.as_ptr() as usize, data.len()); }
	}
	pub fn fill_rect(&self, x: u32, y: u32, w: u32, h: u32, colour: u32) {
		unsafe { self.0.call_5(::values::GUI_WIN_FILLRECT, x as usize, y as usize, w as usize, h as usize, colour as usize); }
	}
}
impl ::Object for Window
{
	const CLASS: u16 = ::values::CLASS_GUI_WIN;
	fn from_handle(handle: super::ObjectHandle) -> Self {
		Window(handle)
	}
	
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

