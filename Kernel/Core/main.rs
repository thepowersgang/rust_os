//
//
//
#![no_std]
extern crate core;
extern crate arch;

#[no_mangle]
pub extern "C" fn kmain()
{
	arch::puts("Hello World");
}

// vim: ft=rust

