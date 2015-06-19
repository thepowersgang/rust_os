// Tifflin OS - Userland loader
// - By John Hodge (thePowersGang)
//
// This program is both the initial entrypoint for the userland, and the default dynamic linker.
#[macro_use]
extern crate tifflin_syscalls;

// Main: This is the initial boot entrypoint
#[no_mangle]
pub extern "C" fn loader_main(cmdline: *const u8, cmdline_len: usize) -> !
{
	kernel_log!("loader_main({:p}, {})", cmdline, cmdline_len);
	let cmdline = unsafe { ::std::str::from_utf8_unchecked( ::std::slice::from_raw_parts(cmdline, cmdline_len) ) };
	// 1. Print the INIT parameter from the kernel
	kernel_log!("- cmdline={:?}", cmdline);
	
	::tifflin_syscalls::exit(0);
}
