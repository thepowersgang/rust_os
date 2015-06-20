// Tifflin OS - Userland loader
// - By John Hodge (thePowersGang)
//
// This program is both the initial entrypoint for the userland, and the default dynamic linker.
#[macro_use]
extern crate tifflin_syscalls;

extern crate cmdline_words_parser;

use cmdline_words_parser::StrExt;

mod elf;

// Main: This is the initial boot entrypoint
#[no_mangle]
pub extern "C" fn loader_main(cmdline: *mut u8, cmdline_len: usize) -> !
{
	kernel_log!("loader_main({:p}, {})", cmdline, cmdline_len);
	// (maybe) SAFE: Need to actually check UTF-8 ness?
	let cmdline: &mut str = unsafe { ::std::mem::transmute( ::std::slice::from_raw_parts_mut(cmdline, cmdline_len) ) };
	// 1. Print the INIT parameter from the kernel
	kernel_log!("- cmdline={:?}", cmdline);
	
	// 2. Parse 'cmdline' into the init path and arguments.
	// TODO: Parse path as an escaped string. Should be able to use a parser that takes &mut str and returns reborrows
	//       - Such a parser would be able to clobber the string as escaping is undone, using the assumption that esc.len >= real.len
	let mut arg_iter = cmdline.parse_cmdline_words();
	let init_path = arg_iter.next().expect("Init path is empty");
	// 3. Spin up init
	// - Open the init path passed in `cmdline`
	let handle = ::elf::load_executable(init_path);
	
	// Populate arguments
	// SAFE: We will be writing to this before reading from it
	let mut args_buf: [&str; 16] = unsafe { ::std::mem::uninitialized() };
	let mut argc = 0;
	for arg in arg_iter {
		args_buf[argc] = arg;
		argc += 1;
	}
	let args = &args_buf[..argc];
	
	// TODO: Switch stacks into a larger dynamically-allocated stack
	let ep: fn(&[&str]) -> ! = handle.get_entrypoint();
	ep(args);
	
	::tifflin_syscalls::exit(0);
}
