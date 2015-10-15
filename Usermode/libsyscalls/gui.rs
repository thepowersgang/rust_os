// Tifflin OS - System Calls
// - By John Hodge (thePowersGang)
//
// gui.rs
pub use ::values::GuiEvent as Event;
pub use ::values::KeyCode as KeyCode;

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
	/// Primary white
	pub fn white() -> Colour { Colour(0xFFFFFF) }

	/// Primary green
	pub fn def_green() -> Colour { Colour(0x00FF00) }
	/// Primary yellow
	pub fn def_yellow() -> Colour { Colour(0xFFFF00) }
}


impl Group
{
	pub fn new(name: &str) -> Result<Group,()>
	{
		// SAFE: Syscall
		match super::ObjectHandle::new( unsafe { syscall!(GUI_NEWGROUP, name.as_ptr() as usize, name.len()) } as usize )
		{
		Ok(rv) => Ok( Group(rv) ),
		Err(code) => {
			panic!("TODO: Error code {}", code);
			},
		}
	}
	
	pub fn force_active(&self) -> Result<(),()> {
		// SAFE: Syscall
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
	fn handle(&self) -> &::ObjectHandle { &self.0 }

	type Waits = GroupWaits;
}
define_waits!{ GroupWaits => (
	showhide:has_showhide = ::values::EV_GUI_GRP_SHOWHIDE,
)}

pub fn set_group(grp: Group)
{
	use Object;
	// SAFE: Syscall
	unsafe { syscall!(GUI_BINDGROUP, grp.into_handle().into_raw() as usize); }
}
pub fn clone_group_handle() -> Group
{
	// SAFE: Syscall with no arguments (... I feel dirty)
	match super::ObjectHandle::new( unsafe { syscall!(GUI_GETGROUP) } as usize )
	{
	Ok(rv) => Group(rv),
	Err(_) => panic!("Attempting to clone GUI group handle when no group registered"),
	}
}

impl Window
{
	pub fn new(name: &str) -> Result<Window,()>
	{
		// SAFE: Syscall
		match super::ObjectHandle::new( unsafe { syscall!(GUI_NEWWINDOW, name.as_ptr() as usize, name.len()) } as usize )
		{
		Ok(rv) => Ok( Window(rv) ),
		Err(code) => {
			panic!("TODO: Error code {}", code);
			},
		}
	}
	
	pub fn show(&self) {
		// SAFE: Syscall
		unsafe { self.0.call_2(::values::GUI_WIN_SETFLAG, ::values::GUI_WIN_FLAG_VISIBLE as usize, 1); }
	}
	pub fn hide(&self) {
		// SAFE: Syscall
		unsafe { self.0.call_2(::values::GUI_WIN_SETFLAG, ::values::GUI_WIN_FLAG_VISIBLE as usize, 0); }
	}
	pub fn redraw(&self) {
		// SAFE: Syscall
		unsafe { self.0.call_0(::values::GUI_WIN_REDRAW); }
	}

	pub fn get_dims(&self) -> Dims {
		// SAFE: No side-effect syscall
		let v = unsafe { self.0.call_0(::values::GUI_WIN_GETDIMS) };
		Dims { w: (v >> 32) as u32, h: v as u32 }
	}
	pub fn set_dims(&self, dims: Dims) {
		// SAFE: Syscall
		unsafe { self.0.call_2(::values::GUI_WIN_SETDIMS, dims.w as usize, dims.h as usize); }
	}

	pub fn get_pos(&self) -> (u32, u32) {
		// SAFE: No side-effect syscall
		let v = unsafe { self.0.call_0(::values::GUI_WIN_GETPOS) };
		( (v >> 32) as u32, v as u32 )
	}
	pub fn set_pos(&self, x: u32, y: u32) {
		// SAFE: Syscall
		unsafe { self.0.call_2(::values::GUI_WIN_SETPOS, x as usize, y as usize); }
	}
	// TODO: Should this be controllable by the application?
	pub fn maximise(&self) {
		// SAFE: Syscall
		unsafe { self.0.call_2(::values::GUI_WIN_SETFLAG, ::values::GUI_WIN_FLAG_MAXIMISED as usize, 1); }
	}
	
	pub fn blit_rect(&self, x: u32, y: u32, w: u32, h: u32, data: &[u32], stride: usize) {
		let rgn_size = if h == 0 { 0 } else { (h - 1) as usize * stride + w as usize };
		assert!( data.len() >= rgn_size );
		let data = &data[..rgn_size];

		// Assert that data length and h*stride agree
		{
			assert!(data.len() > 0);
			let h_calc = if data.len() >= w as usize {
					((data.len() - w as usize) / stride) as u32 + 1
				} else {
					1
				};
			assert_eq!(h_calc, h);
		}

		// SAFE: Syscall
		unsafe { self.0.call_6(::values::GUI_WIN_BLITRECT, x as usize, y as usize, w as usize, data.as_ptr() as usize, data.len(), stride); }
	}
	pub fn fill_rect(&self, x: u32, y: u32, w: u32, h: u32, colour: u32) {
		// SAFE: Syscall
		unsafe { self.0.call_5(::values::GUI_WIN_FILLRECT, x as usize, y as usize, w as usize, h as usize, colour as usize); }
	}

	pub fn pop_event(&self) -> Option<::values::GuiEvent> {
		let mut ev = ::values::GuiEvent::None;
		// SAFE: Syscall
		let v = unsafe { self.0.call_1(::values::GUI_WIN_GETEVENT, &mut ev as *mut _ as usize) };
		if v == !0 {
			None
		}
		else {
			Some( ev )
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
	fn handle(&self) -> &::ObjectHandle { &self.0 }
	
	type Waits = WindowWaits;
}
define_waits!{ WindowWaits => (
	input:has_input = ::values::EV_GUI_WIN_INPUT,
)}

