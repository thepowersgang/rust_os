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
}

