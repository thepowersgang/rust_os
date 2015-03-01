// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/crate.rs
// - AMD64/x86_64 architecture support
extern crate core;
use core::option::Option;

pub use self::log::{puts, puth};

// Emits a distinctive instruction (with no effect)
macro_rules! CHECKMARK{ () => (unsafe { asm!("xchg %cx, %cx" : : : : "volatile");}); }

module_define!{arch, [APIC, HPET], init}

pub mod interrupts;
pub mod memory;
pub mod threads;
pub mod boot;
pub mod sync;

mod log;
pub mod x86_io;
pub mod hw;
pub mod acpi;
pub mod pci;

extern "C"
{
	static v_kernel_end : ();
}

fn init()
{
	// None needed, just dependencies
}

/// Return the system timestamp (miliseconds since an arbitary point)
pub fn cur_timestamp() -> u64
{
	hw::hpet::get_timestamp()
}

/// Print a backtrace, starting at the current location.
pub fn print_backtrace()
{
	let cur_bp: u64;
	unsafe{ asm!("mov %rbp, $0" : "=r" (cur_bp)); }
	puts("Backtrace: ");
	puth(cur_bp);
	
	let mut bp = cur_bp;
	while let Option::Some((newbp, ip)) = interrupts::backtrace(bp)
	{
		puts(" > "); puth(ip);
		bp = newbp;
	}
	puts("\n");
}

// vim: ft=rust

