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

#[allow(improper_ctypes)]
extern "C"
{
	// NOTES:
	// - Required data for spawning a new process:
	//  > Binary path
	//  > Arguments
	//  > ? Environment (could this be transferred using IPC during init?)
	//  > ? Handles (same thing really, send them over an IPC channel)
	pub fn new_process(binary: &[u8], args: &[&[u8]]) -> Result<ProcessHandle,Error>;
}

//#[lang = "panic_fmt"]
//extern "C" fn rust_begin_unwind(msg: ::core::fmt::Arguments, file: &'static str, line: usize) -> ! {
//	//kernel_log!("PANIC: {}:{}: {}", file, line, msg);
//	::tifflin_syscalls::exit(0xFFFF_FFFF);
//}
//#[lang = "eh_personality"]
//fn rust_eh_personality() -> ! {
//	::tifflin_syscalls::exit(0xFFFF_FFFE);
//}
//#[lang = "stack_exhausted"]
//fn stack_exhausted() -> ! {
//	::tifflin_syscalls::exit(0xFFFF_FFFE);
//}

#[no_mangle]
pub fn main() {}
#[no_mangle]
pub fn register_arguments() {}

