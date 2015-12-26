// Tifflin OS - Userland loader interface
// - By John Hodge (thePowersGang)
//
// A dummy interface library that provides dynamically-linked interfaces to the loader
//#![feature(lang_items)]
#![no_std]
#![crate_type="dylib"]
#![crate_name="loader"]

extern crate std_rt;
extern crate syscalls;

use core::result::Result;

pub enum Error
{
	NotFound,
	NotExecutable,
	BadFormat,
	CorruptExecutable,
	BadArguments,
}

mod int {
	use core::result::Result;
	#[allow(improper_ctypes)]
	#[link(name="loader_dyn")]
	extern "C"
	{
		// NOTES:
		// - Required data for spawning a new process:
		//  > Binary path
		//  > Arguments
		//  > ? Environment (could this be transferred using IPC during init?)
		//  > ? Handles (same thing really, send them over an IPC channel)
		pub fn new_process(binary: &[u8], args: &[&[u8]]) -> Result<::syscalls::threads::Process,super::Error>;
	}
}

pub fn new_process(binary: &[u8], args: &[&[u8]]) -> Result<::syscalls::threads::Process,Error> {
	// SAFE: Call is actually to rust
	unsafe { int::new_process(binary, args) }
}

