// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/crate.rs
// - AMD64/x86_64 architecture support
#![macro_escape]

extern crate core;

pub use self::log::{puts, puth};

// Emits a distinctive instruction (with no effect)
macro_rules! CHECKMARK{ () => (unsafe { asm!("xchg %cx, %cx" : : : : "volatile");}); }

pub mod float;
pub mod interrupts;
pub mod memory;
pub mod threads;
pub mod boot;
pub mod sync;

mod log;
mod x86_io;
pub mod hw;
pub mod acpi;

extern "C"
{
	static v_kernel_end : ();
}

pub fn cur_timestamp() -> u64
{
	hw::hpet::get_timestamp()
}

pub fn idle()
{
	unsafe { asm!("hlt"); }
}

// vim: ft=rust

