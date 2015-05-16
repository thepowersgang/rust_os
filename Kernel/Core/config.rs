// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/config.rs
//! Boot-time configuration managment
use prelude::*;

pub enum Value
{
	/// VFS - Volume to mount as the 'system' disk
	SysDisk,
	/// VFS - Path relative to the root of SysDisk where Tifflin was installed
	SysRoot,
}

pub fn get_string(val: Value) -> &'static str
{
	match val
	{
	Value::SysDisk => "ATA-0p0",
	Value::SysRoot => "/Tifflin/",
	}
}

