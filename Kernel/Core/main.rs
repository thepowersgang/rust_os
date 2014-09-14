// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/main.rs
// - Kernel main
#![no_std]
#![feature(phase)]
#![feature(macro_rules,asm)]
#![feature(unsafe_destructor)]
#![feature(thread_local)]
#![feature(globs)]

#[phase(plugin, link)] extern crate core;
#[phase(plugin, link)] extern crate common;
//#[phase(plugin, link)] extern crate arch;

use _common::*;

pub use arch::memory::PAGE_SIZE;

pub mod logmacros;

#[cfg(arch__amd64)]
#[path="../arch/amd64/crate.rs"]
pub mod arch;	// Needs to be pub for exports to be avaliable

// Evil Hack: For some reason, write! (and friends) will expand pointing to std instead of core
mod std { pub use core::{default,fmt,cmp}; }
mod _common;

mod lib;	// Clone of libstd
mod sync;
mod logging;
mod memory;
mod threads;
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
	::threads::init();
	
	log_log!("Command line = '{}'", ::arch::boot::get_boot_string());
	//::devices::display::init();
	
	// Dump active video mode
	let vidmode = ::arch::boot::get_video_mode();
	match vidmode {
	Some(m) => log_debug!("Video mode : {}x{}", m.width, m.height),
	None => log_debug!("No video mode present")
	}
	
	loop
	{
		::threads::reschedule();
		::arch::idle();
	}
}

// Evil fail when doing unwind
//#[lang="begin_unwind"] fn rust_begin_unwind(msg: &::core::fmt::Arguments, file: &'static str, line: uint) -> !
#[no_mangle] pub extern "C" fn rust_begin_unwind(msg: &::core::fmt::Arguments, file: &'static str, line: uint) -> !
{
	arch::puts("ERROR: rust_begin_unwind\n");
	log_panic!("rust_begin_unwind(msg=\"{}\", file=\"{}\", line={})", msg, file, line);
	loop{}
}

// vim: ft=rust

