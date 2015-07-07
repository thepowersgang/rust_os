// Tifflin OS - Userland loader interface
// - By John Hodge (thePowersGang)
//
// A dummy interface library that provides dynamically-linked interfaces to the loader
#![feature(no_std,lang_items,core)]
#![no_std]
#![crate_type="dylib"]
#![crate_name="loader"]

extern crate core;
extern crate std_rt;

use core::result::Result;

pub struct ProcessHandle(u32);

pub enum Error
{
	NotFound,
	NotExecutable,
	BadFormat,
	CorruptExecutable,
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
		pub fn new_process(binary: &[u8], args: &[&[u8]]) -> Result<super::ProcessHandle,super::Error>;
	}
}

pub fn new_process(binary: &[u8], args: &[&[u8]]) -> Result<ProcessHandle,Error> {
	// SAFE: Call is actually to rust
	unsafe { int::new_process(binary, args) }
}

