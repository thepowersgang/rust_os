//
//
//
#![no_std]
#![feature(phase)]
#![feature(macro_rules)]

#[phase(plugin, link)] extern crate core;
extern crate arch;

// Evil Hack: For some reason, write! (and friends) will expand pointing to std instead of core
mod std { pub use core::fmt; }

mod logging;
mod time;

#[no_mangle]
pub extern "C" fn kmain()
{
	arch::puts("Hello World\n");
	log_notice!("Tifflin Kernel starting");
}

// Evil fail when doing unwind
#[no_mangle]
pub extern "C" fn rust_begin_unwind()
{
	arch::puts("ERROR: rust_begin_unwind\n");
	loop{}
}

// vim: ft=rust

