// Tifflin OS - Standard Library Runtime
// - By John Hodge (thePowersGang)
//
// Standard Library - Runtime support (aka unwind and panic)
#![feature(no_std,core,core_prelude,core_intrinsics)]
#![feature(lang_items)]	// Allow definition of lang_items
#![no_std]

use core::prelude::*;

#[macro_use]
extern crate core;
#[macro_use]
extern crate tifflin_syscalls;

mod std {
	pub use core::fmt;
}

pub fn begin_unwind<M: ::core::any::Any+Send+'static>(msg: M, file_line: &(&'static str, u32)) -> ! {
	let file = file_line.0;
	let line = file_line.1 as usize;
	if let Some(m) = ::core::any::Any::downcast_ref::<::core::fmt::Arguments>(&msg) {
		rust_begin_unwind(format_args!("{}", m), file, line)
	}
	else if let Some(m) = ::core::any::Any::downcast_ref::<&str>(&msg) {
		rust_begin_unwind(format_args!("{}", m), file, line)
	}
	else {
		rust_begin_unwind(format_args!("begin_unwind<{}>", unsafe { ::core::intrinsics::type_name::<M>() }), file, line)
	}
}
pub fn begin_unwind_fmt(msg: ::core::fmt::Arguments, file_line: &(&'static str, u32)) -> ! {
	rust_begin_unwind(msg, file_line.0, file_line.1 as usize)
}

#[lang = "panic_fmt"]
pub extern "C" fn rust_begin_unwind(msg: ::core::fmt::Arguments, file: &'static str, line: usize) -> ! {
	use core::fmt::Write;
	// Spit out that log
	kernel_log!("PANIC: {}:{}: {}", file, line, msg);
	// Exit the process with a special error code
	::tifflin_syscalls::exit(0xFFFF_FFFF);
}
#[lang="eh_personality"]
fn rust_eh_personality(
	//version: isize, _actions: _Unwind_Action, _exception_class: u64,
	//_exception_object: &_Unwind_Exception, _context: &_Unwind_Context
	)// -> _Unwind_Reason_Code
{
	loop {} 
}

#[lang = "stack_exhausted"]
extern fn stack_exhausted() {
	loop {}
}



