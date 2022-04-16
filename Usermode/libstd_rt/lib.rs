// Tifflin OS - Standard Library Runtime
// - By John Hodge (thePowersGang)
//
// Standard Library - Runtime support (aka unwind and panic)
#![feature(lang_items)]	// Allow definition of lang_items
#![feature(panic_info_message)]
#![cfg_attr(target_arch="arm", feature(extern_types))]	// Used for ARM unwind
#![no_std]

#[macro_use]
extern crate syscalls;
#[allow(unused_imports)]
#[macro_use]
extern crate macros;

mod std {
	pub use core::fmt;
}

/// Helpers so ARM can include the kernel's backtrace code
#[cfg(target_arch="arm")]
mod memory {
	pub mod virt {
		pub fn is_reserved<T>(_p: *const T) -> bool {
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

#[cfg(arch="native")]
#[path="arch-native.rs"]
mod arch;
#[cfg(not(arch="native"))]
#[cfg_attr(target_arch="x86_64", path="arch-x86_64.rs")]
#[cfg_attr(target_arch="arm", path="arch-armv7.rs")]
#[cfg_attr(target_arch="aarch64", path="arch-armv8.rs")]
#[cfg_attr(target_arch="riscv64", path="arch-riscv64.rs")]
mod arch;

#[cfg_attr(test,allow(dead_code))]
fn begin_panic_fmt(msg: &::core::fmt::Arguments, file_line: (&str, u32)) -> ! {
	// Spit out that log
	kernel_log!("PANIC: {}:{}: {}", file_line.0, file_line.1, msg);
	// - Backtrace
	let bt = arch::Backtrace::new();
	kernel_log!("- {} Backtrace: {:?}", file_line.0, bt);
	// Exit the process with a special error code
	::syscalls::threads::exit(0xFFFF_FFFF);
}

#[panic_handler]
#[cfg(not(test))]
pub extern fn rust_begin_unwind(info: &::core::panic::PanicInfo) -> ! {
	let file_line = match info.location()
		{
		Some(v) => (v.file(), v.line()),
		None => ("", 0),
		};
	if let Some(m) = info.payload().downcast_ref::<::core::fmt::Arguments>() {
		begin_panic_fmt(m, file_line)
	}
	else if let Some(m) = info.payload().downcast_ref::<&str>() {
		begin_panic_fmt(&format_args!("{}", m), file_line)
	}
	else if let Some(m) = info.message() {
		begin_panic_fmt(m, file_line)
	}
	else {
		begin_panic_fmt(&format_args!("Unknown"), file_line)
	}
}
#[lang="eh_personality"]
#[cfg(not(test))]
fn rust_eh_personality(
	//version: isize, _actions: _Unwind_Action, _exception_class: u64,
	//_exception_object: &_Unwind_Exception, _context: &_Unwind_Context
	)// -> _Unwind_Reason_Code
{
	loop {} 
}


