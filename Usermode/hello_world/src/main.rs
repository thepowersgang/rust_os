#![no_std]
#![no_main]

extern crate syscalls;
extern crate std_rt;

#[no_mangle]
extern "C" fn main(_: isize, _: *const *const u8) -> isize {
	::syscalls::log_write("Hello World!");
	0
}

#[no_mangle]
pub extern "C" fn register_arguments() {
	// Does nothing
}

//#[no_mangle]
//pub extern "C" fn _Unwind_Resume() {
//	// Does nothing
//}

