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
	static v_kernel_end: crate::Extern;
}

fn init()
{
	pci::init();
}

#[inline(always)]
pub fn checkmark() {
	// SAFE: nop ASM
	unsafe { ::core::arch::asm!("xchg bx, bx", options(nostack)); }
}
#[inline(always)]
pub fn checkmark_val<T>(v: *const T) {
	// SAFE: nop ASM (TODO: Ensure)
	unsafe { ::core::arch::asm!("xchg bx, bx; mov {0},{0}", in(reg) v, options(nostack)); }
}

#[allow(improper_ctypes)]
extern "C" {
	pub fn drop_to_user(entry: usize, stack: usize, cmdline_len: usize) -> !;
}

pub mod time {
	/// Return the system timestamp (miliseconds since an arbitary point)
	pub fn cur_timestamp() -> u64
	{
		super::hw::hpet::get_timestamp()
	}
	
	pub fn request_tick(target_time: u64)
	{
		super::hw::hpet::request_tick(target_time)
	}
}

/// Print a backtrace, starting at the current location.
pub fn print_backtrace()
{
	let cur_bp: u64;
	// SAFE: Reads from bp
	unsafe{ ::core::arch::asm!("mov {}, rbp", out(reg) cur_bp); }
	#[cfg(_false)]
	log_notice!("Backtrace: {}", Backtrace(cur_bp as usize));
	#[cfg(not(_false))]
	{
		let mut bp = cur_bp as u64;
		while let Option::Some((newbp, ip)) = cpu_faults::backtrace(bp)
		{
			log_notice!("> {}", SymPrint(ip as usize));
			bp = newbp;
		}
	}
}
// TODO: Put this somewhere common (in `symbols` maybe?)
struct SymPrint(usize);
impl ::core::fmt::Display for SymPrint
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		let ip = self.0;
		write!(f, "{:#x}", ip)?;
		if let Some( (name, ofs) ) = crate::symbols::get_symbol_for_addr(ip as usize - 1) {
			write!(f, "({}+{:#x})", crate::symbols::Demangle(name), ofs + 1)?;
		}
		Ok( () )
	}
}
pub struct Backtrace(usize);
impl ::core::fmt::Display for Backtrace {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		let mut bp = self.0 as u64;
		while let Option::Some((newbp, ip)) = cpu_faults::backtrace(bp)
		{
			write!(f, " > {}", SymPrint(ip as usize))?;
			bp = newbp;
		}
		Ok( () )
	}
}

// vim: ft=rust

