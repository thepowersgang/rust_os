// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/crate.rs
// - AMD64/x86_64 architecture support
extern crate core;
use core::option::Option;

pub use self::log::{puts, puth};

module_define!{arch, [APIC, HPET], init}

pub mod interrupts;
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
	static v_kernel_end : ();
}

fn init()
{
	// None needed, just dependencies
}

#[allow(improper_ctypes)]
extern "C" {
	pub fn drop_to_user(entry: usize, cmdline_len: usize) -> !;
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

#[no_mangle]
pub extern "C" fn syscalls_handler(id: u32, first_arg: *const usize, count: u32) -> u64
{
	//log_debug!("syscalls_handler({}, {:p}+{})", id, first_arg, count);
	::syscalls::invoke(id, unsafe { ::core::slice::from_raw_parts(first_arg, count as usize) })
}

// vim: ft=rust

