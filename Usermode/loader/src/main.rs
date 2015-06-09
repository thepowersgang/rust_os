// Tifflin OS - Userland loader
// - By John Hodge (thePowersGang)
//
// This program is both the initial entrypoint for the userland, and the default dynamic linker.
extern crate tifflin_syscalls;

// Main: This is the initial boot entrypoint
#[no_mangle]
pub extern "C" fn loader_main(cmdline: *const str) -> !
{
	use std::fmt::Write;
	let cmdline = unsafe { &*cmdline };
	// 1. Request INIT parameter from the kernel
	// - Remove the path to this binary.
	// TODO: Maybe this can be passed in a buffer provided in the image?
	// Spit out that log
	let _ = write!(&mut ::tifflin_syscalls::ThreadLogWriter, "loader_main(cmdline=\"{}\")", cmdline);
	
	loop {}
}
