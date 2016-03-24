// Tifflin OS - Standard Library Runtime
// - By John Hodge (thePowersGang)
//
// Standard Library - Runtime support (aka unwind and panic)
#![feature(lang_items)]	// Allow definition of lang_items
#![feature(asm)]	// Used by backtrace code
#![feature(const_fn)]	// HACK For debugging ARM unwind
#![no_std]

#[macro_use]
extern crate syscalls;
#[macro_use]
extern crate macros;

mod std {
	pub use core::fmt;
}

#[cfg(arch="armv7")]
enum Void {}
#[cfg(arch="armv7")]
mod memory {
	pub mod virt {
		pub fn is_reserved<T>(p: *const T) -> bool {
			true
		}
	}
	pub unsafe fn buf_to_slice<'a, T: 'a>(ptr: *const T, count: usize) -> Option<&'a [T]> {
		if virt::is_reserved(ptr) {
			Some(::core::slice::from_raw_parts(ptr, count))
		}
		else {
			None
		}
	}
}

#[cfg(arch="amd64")]
#[path="arch-x86_64.rs"]
mod arch;
#[cfg(arch="armv7")]
#[path="arch-armv7.rs"]
mod arch;

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
		rust_begin_unwind(format_args!("begin_unwind<{}>", type_name!(M)), file, line)
	}
}
pub fn begin_unwind_fmt(msg: ::core::fmt::Arguments, file_line: &(&'static str, u32)) -> ! {
	rust_begin_unwind(msg, file_line.0, file_line.1 as usize)
}

#[lang = "panic_fmt"]
#[no_mangle]
pub extern "C" fn rust_begin_unwind(msg: ::core::fmt::Arguments, file: &'static str, line: usize) -> ! {
	use core::fmt::Write;
	// Spit out that log
	kernel_log!("PANIC: {}:{}: {}", file, line, msg);
	// - Backtrace
	let bt = arch::Backtrace::new();
	kernel_log!("- {} Backtrace: {:?}", file, bt);
	// Exit the process with a special error code
	::syscalls::threads::exit(0xFFFF_FFFF);
}
#[lang="eh_personality"]
fn rust_eh_personality(
	//version: isize, _actions: _Unwind_Action, _exception_class: u64,
	//_exception_object: &_Unwind_Exception, _context: &_Unwind_Context
	)// -> _Unwind_Reason_Code
{
	loop {} 
}


