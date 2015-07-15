// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/gui.rs
/// GUI syscall interface
use prelude::*;

use super::{objects,ObjectHandle};
use super::values;
use super::Error;

pub fn newgroup(name: &str) -> Result<ObjectHandle,u32> {
	todo!("syscall_gui_newgroup(name={})", name);
}
pub fn newwindow(name: &str) -> Result<ObjectHandle,u32> {
	todo!("syscall_gui_newwindow(name={})", name);
}

