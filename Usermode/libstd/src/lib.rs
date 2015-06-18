//
//
//
#![crate_type="rlib"]
#![crate_name="std"]
#![feature(no_std,core)]
#![feature(lang_items)]	// Allow definition of lang_items
#![no_std]

#[macro_use]
extern crate core;
extern crate tifflin_syscalls;

use core::prelude::*;

// Raw re-exports from core
pub use core::fmt;
pub use core::slice;
pub use core::str;

/// Prelude
pub mod prelude {
	pub mod v1 {
		pub use core::prelude::*;
	}
}


#[lang = "panic_fmt"]
pub extern "C" fn rust_begin_unwind(msg: ::core::fmt::Arguments, file: &'static str, line: usize) -> ! {
	use core::fmt::Write;
	// Spit out that log
	let _ = write!(&mut ::tifflin_syscalls::ThreadLogWriter, "PANIC: {}:{}: {}", file, line, msg);
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

