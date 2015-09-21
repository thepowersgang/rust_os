// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/crate.rs
// - AMD64/x86_64 architecture support
use core::option::Option;

pub use self::log::{puts, puth};

module_define!{arch, [APIC, HPET], init}

pub mod interrupts;
#[doc(hidden)]
pub mod cpu_faults;
pub mod memory;
pub mod threads;
pub mod boot;
pub mod sync;

mod tss;

mod log;
pub mod x86_io;
pub mod hw;
pub mod acpi;
pub mod pci;

extern "C"
{
	static v_kernel_end : ::Void;
}

fn init()
{
	// None needed, just dependencies
}

#[allow(improper_ctypes)]
extern "C" {
	pub fn drop_to_user(entry: usize, stack: usize, cmdline_len: usize) -> !;
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
	// SAFE: Reads from bp
	unsafe{ asm!("mov %rbp, $0" : "=r" (cur_bp)); }
	puts("Backtrace: ");
	puth(cur_bp);
	
	let mut bp = cur_bp;
	while let Option::Some((newbp, ip)) = cpu_faults::backtrace(bp)
	{
		puts(" > "); puth(ip);
		bp = newbp;
	}
	puts("\n");
}

// vim: ft=rust

