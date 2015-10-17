// "Tifflin" Operating System - Window Toolkit
// - By John Hodge (thePowersGang)
//
// libwtk/menu.rs
//! Pop-up menu support

pub struct OpenMenu
{
}

pub struct Menu
{
	window: ::syscalls::gui::Window,
	items: Vec<Option<MenuItem>>,
}

struct MenuItem
{
	label: String,
	accel_ofs: usize,
	altlabel: String,
	
	value: usize,
	action: Box<Fn()>,
}

