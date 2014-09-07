// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/main.rs
// - Kernel main
#![no_std]
#![feature(phase)]
#![feature(macro_rules)]

#[phase(plugin, link)] extern crate core;
extern crate common;
extern crate arch;

use core::option::{Some,None};

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
	
	log_log!("Command line = '{}'", ::arch::boot::get_boot_string());
	//::devices::display::init();
	let vidmode = ::arch::boot::get_video_mode();
	match vidmode {
	Some(m) => log_debug!("Video mode : {}x{}", m.width, m.height),
	None => log_debug!("No video mode present")
	}
}

// Evil fail when doing unwind
#[no_mangle]
pub extern "C" fn rust_begin_unwind()
{
	arch::puts("ERROR: rust_begin_unwind\n");
	loop{}
}

// vim: ft=rust

