// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/main.rs
// - Kernel main
#![crate_name="kernel"]
#![crate_type="lib"]
#![no_std]
#![feature(asm)]
#![feature(box_syntax)]
#![feature(unsafe_destructor)]
#![feature(thread_local)]
#![feature(lang_items)]

#[macro_use]
#[macro_reexport(assert,panic,write)]
extern crate core;

use _common::*;

pub use arch::memory::PAGE_SIZE;

#[macro_use] pub mod logmacros;
#[macro_use] pub mod macros;
#[macro_use] #[cfg(arch__amd64)] #[path="arch/amd64/macros.rs"] pub mod arch_macros;	// Needs to be pub for exports to be avaliable

// Evil Hack: For some reason, write! (and friends) will expand pointing to std instead of core
mod std {
	pub use core::option;
	pub use core::{default,fmt,cmp};
	pub use lib::clone;
	pub use core::marker;	// needed for derive(Copy)
}
pub mod _common;

#[macro_use]
pub mod lib;	// Clone of libstd
#[macro_use]
pub mod sync;

pub mod logging;
pub mod memory;
pub mod threads;
pub mod time;
pub mod modules;

pub mod metadevs;
pub mod hw;
pub mod device_manager;

pub mod unwind;

#[macro_use]
#[cfg(arch__amd64)] #[path="arch/amd64/crate.rs"] pub mod arch;	// Needs to be pub for exports to be avaliable

#[no_mangle]
pub extern "C" fn kmain()
{
	log_notice!("Tifflin Kernel v{} build {} starting", env!("TK_VERSION"), env!("TK_BUILD"));
	log_notice!("> Git state : {}", env!("TK_GITSPEC"));
	log_notice!("> Built with {}", env!("RUST_VERSION"));
	
	// Initialise core services before attempting modules
	::memory::phys::init();
	::memory::virt::init();
	::memory::heap::init();
	::threads::init();
	
	log_log!("Command line = '{}'", ::arch::boot::get_boot_string());
	
	// Modules (dependency tree included)
	::modules::init();
	
	// Dump active video mode
	let vidmode = ::arch::boot::get_video_mode();
	match vidmode {
	Some(m) => {
		log_debug!("Video mode : {}x{} @ {:#x}", m.width, m.height, m.base)
		// TODO: Create a binding for metadevs::video to handle this mode
		},
	None => log_debug!("No video mode present")
	}
	
	// Thread 0 idle loop
	log_info!("Entering idle");
	loop
	{
		::threads::yield_time();
		log_trace!("TID0 napping");
		::arch::idle();
	}
}

#[no_mangle] pub unsafe extern "C" fn malloc(size: usize) -> *mut () {
	memory::heap::allocate(memory::heap::HeapId::Global, size).unwrap()
} 
#[no_mangle] pub unsafe extern "C" fn free(ptr: *mut ()) {
	use core::ptr::PtrExt;
	if !ptr.is_null() { memory::heap::deallocate(ptr) }
} 


// TODO: Move out
pub mod common
{
pub mod archapi
{

#[derive(Copy)]
pub enum VideoFormat
{
	X8R8G8B8,
	B8G8R8X8,
	R8G8B8,
	B8G8R8,
	R5G6B5,
}

#[derive(Copy)]
pub struct VideoMode
{
	pub width: u16,
	pub height: u16,
	pub fmt: VideoFormat,
	pub pitch: usize,
	pub base: ::arch::memory::PAddr,
}

}
}


// vim: ft=rust

