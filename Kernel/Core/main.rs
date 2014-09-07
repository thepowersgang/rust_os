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
mod memory;
mod time;

#[no_mangle]
pub extern "C" fn kmain()
{
	log_notice!("Tifflin Kernel v{} build {} starting", env!("TK_VERSION"), env!("TK_BUILD"));
	log_notice!("> Git state : {}", env!("TK_GITSPEC"));
	log_notice!("> Built with {}", env!("RUST_VERSION"));
	
	::memory::phys::init();
	::memory::virt::init();
	::memory::heap::init();
	
	//::devices::display::init();
}

// Evil fail when doing unwind
#[no_mangle]
pub extern "C" fn rust_begin_unwind()
{
	arch::puts("ERROR: rust_begin_unwind\n");
	loop{}
}

// vim: ft=rust

