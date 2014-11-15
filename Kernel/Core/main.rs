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
#![feature(concat_idents)]
#![feature(lang_items)]
#![feature(while_let)]
#![feature(tuple_indexing)]

#[phase(plugin, link)] extern crate core;

use _common::*;

pub use arch::memory::PAGE_SIZE;

pub mod logmacros;
pub mod macros;
#[cfg(arch__amd64)] #[path="../arch/amd64/macros.rs"] pub mod arch_macros;	// Needs to be pub for exports to be avaliable

// Evil Hack: For some reason, write! (and friends) will expand pointing to std instead of core
mod std {
	pub use core::{default,fmt,cmp};
	pub use lib::clone;
}
mod _common;

pub mod lib;	// Clone of libstd
mod sync;
mod logging;
pub mod memory;
pub mod threads;
mod time;
pub mod modules;

pub mod unwind;

#[cfg(arch__amd64)] #[path="../arch/amd64/crate.rs"] pub mod arch;	// Needs to be pub for exports to be avaliable

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
	Some(m) => log_debug!("Video mode : {}x{}", m.width, m.height),
	None => log_debug!("No video mode present")
	}
	
	// Thread 0 idle loop
	loop
	{
		::threads::yield_time();
		log_trace!("TID0 napping");
		::arch::idle();
	}
}

#[no_mangle] pub unsafe extern "C" fn malloc(size: uint) -> *mut () {
	memory::heap::allocate(memory::heap::GlobalHeap, size).unwrap()
} 
#[no_mangle] pub unsafe extern "C" fn free(ptr: *mut ()) {
	use core::ptr::RawPtr;
	if !ptr.is_null() { memory::heap::deallocate(ptr) }
} 


// TODO: Move out
pub mod common
{
pub mod archapi
{

pub enum VideoFormat
{
	VideoX8R8G8B8,
	VideoB8G8R8X8,
	VideoR8G8B8,
	VideoB8G8R8,
	VideoR5G6B5,
}

pub struct VideoMode
{
	pub width: u16,
	pub height: u16,
	pub fmt: VideoFormat,
}

}
}


// vim: ft=rust

