// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/arch/armv8/mod.rs
// - ARMv8 (AArch64) interface
use ::core::sync::atomic::{AtomicUsize};

pub mod memory;
pub mod sync;
pub mod threads;
pub mod boot;
pub mod interrupts;

module_define!{arch, [], init}
fn init()
{
	// Start the FDT bus enumeration, informing it of the interrupt controller
	fdt_devices::init(interrupts::get_intc);
}

#[path="../armv7/gic.rs"]
mod gic;
#[path="../armv7/fdt_devices.rs"]
mod fdt_devices;

#[no_mangle]
static CPU0_STATE: CpuState = CpuState {
	current_thread: AtomicUsize::new(0),
	idle_thread: AtomicUsize::new(0),
	};
struct CpuState
{
	current_thread: AtomicUsize,
	idle_thread: AtomicUsize,
}
impl CpuState
{
	fn cur() -> &'static CpuState {
		// SAFE: Reads a register
		unsafe {
			let ret: *const CpuState;
			::core::arch::asm!("mrs {}, TPIDR_EL1", out(reg) ret, options(nomem, pure));
			&*ret
		}
	}
}

pub fn print_backtrace() {
	let mut fp: *const FrameEntry;
	// SAFE: Just loads the frame pointer
	unsafe { ::core::arch::asm!("mov {}, fp", out(reg) fp); }

	#[repr(C)]
	struct FrameEntry {
		next: *const FrameEntry,
		ret_addr: usize,
	}
	puts("Backtrace:");
	while ! fp.is_null()
	{
		if ! crate::memory::virt::is_reserved(fp) {
			break;
		}
		// SAFE: Checked by above
		let data = unsafe { &*fp };
		puts(" -> "); puth(data.ret_addr as u64);
		if let Some( (name,ofs) ) = crate::symbols::get_symbol_for_addr(data.ret_addr) {
			puts("("); puts(name); puts("+"); puth(ofs as u64); puts(")");
		}
		fp = data.next;
	}
	puts("\n");
}

pub mod time {
	pub fn cur_timestamp() -> u64 {
		0
	}
	pub fn request_tick(time: u64) {
		todo!("request_tick");
	}
}

pub unsafe fn drop_to_user(entry: usize, stack: usize, args_len: usize) -> ! {
	extern "C" {
		fn drop_to_user(entry: usize, stack: usize, args_len: usize) -> !;
	}
	//crate::logging::hex_dump("drop_to_user", ::core::slice::from_raw_parts(0x7feffffd23a1 as *const u8, 11));
	drop_to_user(entry, stack, args_len);
}


pub mod x86_io {
	pub unsafe fn inb(_p: u16) -> u8 { panic!("calling inb on ARM") }
	pub unsafe fn inw(_p: u16) -> u16 { panic!("calling inw on ARM") }
	pub unsafe fn inl(_p: u16) -> u32 { panic!("calling inl on ARM") }
	pub unsafe fn outb(_p: u16, _v: u8) {}
	pub unsafe fn outw(_p: u16, _v: u16) {}
	pub unsafe fn outl(_p: u16, _v: u32) {}
}


fn putb(b: u8) {
	// SAFE: Access should be correct, and no race is possible
	unsafe {
		// - First HWMap page is the UART
		let uart = memory::addresses::HARDWARE_BASE as *mut u8;
		::core::intrinsics::volatile_store( uart.offset(0), b );
	}
}
#[inline(never)]
#[no_mangle]
pub fn puts(s: &str) {
	for b in s.bytes() {
		putb(b);
	}
}
#[inline(never)]
#[no_mangle]
pub fn puth(v: u64) {
	putb(b'0');
	putb(b'x');
	if v == 0 {
		putb(b'0');
	}
	else {
		for i in (0 .. 16).rev() {
			if v >> (i * 4) > 0 {
				let n = ((v >> (i * 4)) & 0xF) as u8;
				if n < 10 {
					putb( b'0' + n );
				}
				else {
					putb( b'a' + n - 10 );
				}
			}
		}
	}
}

 
#[repr(C)]
struct Regs
{
	elr: u64,
	spsr: u64,
	/// Caller-saved registers
	saved: [u64; 18],
	fp: u64,
	lr: u64,
}

#[no_mangle]
extern "C" fn vector_handler_irq()
{
	interrupts::handle();
}
#[no_mangle]
extern "C" fn vector_handler_fiq()
{
	todo!("vector_handler_fiq");
}
#[no_mangle]
extern "C" fn vector_handler_sync_u64(esr: u64, regs: &mut Regs)
{
	match (esr >> 26) & 0x3F
	{
	0x15 => {	// SVC from AArch64 state
		extern "C" {
			fn syscalls_handler(id: u32, first_arg: *const usize, count: u32) -> u64;
		}
		// SAFE: Correct FFI signature for Modules/syscalls
		regs.saved[0] = unsafe { syscalls_handler(regs.saved[12] as u32, regs.saved.as_ptr() as *const usize, 6) };
		},
	0x24 => {	// Data abort from lower exception level
		// SAFE: Reads a non-sideeffect register
		let far = unsafe { let v: u64; ::core::arch::asm!("mrs {}, FAR_EL1", lateout(reg) v); v };
		if self::memory::virt::data_abort(esr & ((1<<25)-1), far as usize)
		{
			return ;
		}
		todo!("vector_handler_sync_u64: Data abort {:#x} unhandled", far);
		},
	0x3c => todo!("vector_handler_sync_u64: User BRK instruction: {:#x}", regs.elr),
	ec @ _ => todo!("vector_handler_sync_u64: EC=0x{:x}", ec),
	}
}
#[no_mangle]
extern "C" fn vector_handler_sync_k(esr: u64, regs: &mut Regs)
{
	puts("vector_handler_sync_k: esr="); puth(esr); puts(" ELR="); puth(regs.elr); puts("\n");
	match (esr >> 26) & 0x3F
	{
	ec @ _ => todo!("vector_handler_sync_k: EC=0x{:x} ELR={:#x}", ec, regs.elr),
	}
}

