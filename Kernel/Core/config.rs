// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/config.rs
//! Boot-time configuration managment
#[allow(unused_imports)]
use prelude::*;

pub enum Value
{
	/// VFS - Volume to mount as the 'system' disk
	SysDisk,
	/// VFS - Path relative to the root of SysDisk where Tifflin was installed
	SysRoot,

	/// Startup - Loader executable
	Loader,
	/// Startup - Init executable (first userland process)
	Init,
}

static mut S_SYSDISK: Option<&'static str> = None;
static mut S_SYSROOT: Option<&'static str> = None;
static mut S_INIT: Option<&'static str> = None;
static mut S_LOADER: Option<&'static str> = None;

pub fn init(cmdline: &'static str)
{
	for ent in cmdline.split(' ')
	{
		let mut it = ent.splitn(2, '=');
		let tag = it.next().unwrap();
		let value = it.next();
		match tag
		{
		"SYSDISK" =>
			match value
			{
			// SAFE: Called in single-threaded context
			Some(v) => unsafe { S_SYSDISK = Some(v); },
			None => log_warning!("SYSDISK requires a value"),
			},
		"SYSROOT" =>
			match value
			{
			// SAFE: Called in single-threaded context
			Some(v) => unsafe { S_SYSROOT = Some(v); },
			None => log_warning!("SYSDISK requires a value"),
			},
		v @ _ => log_warning!("Unknown option '{}", v),
		}
	}
}

pub fn get_string(val: Value) -> &'static str
{
	// SAFE: No mutation should happen when get_string is being called
	unsafe {
		match val
		{
		Value::SysDisk => S_SYSDISK.unwrap_or("ATA-0p0"),
		Value::SysRoot => S_SYSROOT.unwrap_or("/system/Tifflin"),
		Value::Init   => S_INIT.unwrap_or("/sysroot/bin/init"),
		Value::Loader => S_LOADER.unwrap_or("/sysroot/bin/loader"),
		}
	}
}

