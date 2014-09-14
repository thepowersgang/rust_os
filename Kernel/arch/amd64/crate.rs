// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/crate.rs
// - AMD64/x86_64 architecture support
#![feature(asm)]
#![crate_type="lib"]
#![no_std]
#![feature(unsafe_destructor)]	// Used to allow type parametered Drop
#![feature(macro_rules)]
#![macro_escape]

extern crate core;
extern crate common;

pub use self::log::{puts, puth};

pub mod float;
pub mod interrupts;
pub mod memory;
pub mod threads;
pub mod boot;
pub mod sync;

mod log;
mod x86_io;

extern "C"
{
	static v_kernel_end : ();
}

pub fn idle()
{
	unsafe { asm!("hlt"); }
}

// vim: ft=rust

