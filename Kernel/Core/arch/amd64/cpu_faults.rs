// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/cpu_faults.rs
//! Fault handlers
//use prelude::*;
use super::{puts,puth};
use super::log::puth64;

#[repr(C)]
//#[derive(Copy,Clone)]
/// Register state as saved by the error ISR handler
pub struct InterruptRegs
{
	//fs: u64,
	//gs: u64,

	rax: u64, rcx: u64, rdx: u64, rbx: u64,
	/*no rsp*/rbp: u64, rsi: u64, rdi: u64,
	r8: u64,  r9: u64,  r10: u64, r11: u64,
	r12: u64, r13: u64, r14: u64, r15: u64,
	
	intnum: u64, errorcode: u64,
	rip: u64, cs: u64,
	rflags: u64, rsp: u64, ss: u64,
}

#[no_mangle]
#[doc(hidden)]
/// Error handler called by assembly
pub extern "C" fn spurrious_handler(regs: &InterruptRegs)
{
	panic!("Spurrious interrupt: v={} @ ip={:#x}", regs.intnum, regs.errorcode);
}

#[no_mangle]
#[doc(hidden)]
/// Error handler called by assembly
pub extern "C" fn error_handler(regs: &InterruptRegs)
{
	// If the fault originated in kernel mode, emit a mode reset
	//if regs.cs == 0x8 {
	//	puts("\x1b[m");
	//}
	match regs.intnum
	{
	7 => {
		// Coprocessor not ready
		if regs.cs == 0x8 {
			puts("Invalid use of coprocessor in kernel mode: IP="); puth(regs.rip); puts("\n");
			loop {}
		}

		puts("#NM at "); puth(regs.rip); puts("\n");
		if super::threads::enable_sse_and_restore() {
			// SSE was disabled, try again with it enabled
			return ;
		}
		else {
			todo!("What should be done if #NM is generated but SSE was already enababled");
		}
		},
	13 => { puts("GPF ("); puth(regs.errorcode); puts(")\n"); },
	14 => {
		let cr2 = get_cr2();
		puts("PF ("); puth(regs.errorcode); puts(") at "); puth(cr2 as u64); puts(" by "); puth(regs.rip); puts(" SP="); puth(regs.rsp); puts("\n");
		if crate::arch::amd64::memory::virt::handle_page_fault(cr2 as usize, regs.errorcode as u32) {
			return ;
		}
		},
	_ => { puts("ERROR "); puth(regs.intnum); puts(" (code "); puth(regs.errorcode); puts(")\n"); },
	}
	puts("CS:RIP  = "); puth(regs.cs); puts(":"); puth(regs.rip); puts("\n");
	puts("SS:RSP  = "); puth(regs.ss); puts(":"); puth(regs.rsp); puts("\n");
	puts("CR2 = "); puth(get_cr2()); puts("\n");
	puts("RAX "); puth64(regs.rax); puts("  RCX "); puth64(regs.rcx); puts("  ");
	puts("RDX "); puth64(regs.rdx); puts("  RBX "); puth64(regs.rbx); puts("\n");
	puts("RSI "); puth64(regs.rsi); puts("  RDI "); puth64(regs.rdi); puts("  ");
	puts("RSP "); puth64(regs.rsp); puts("  RBP "); puth64(regs.rbp); puts("\n");
	puts("R8  "); puth64(regs.r8 ); puts("  R9  "); puth64(regs.r9 ); puts("  ");
	puts("R10 "); puth64(regs.r10); puts("  R11 "); puth64(regs.r11); puts("\n");
	puts("R12 "); puth64(regs.r12); puts("  R13 "); puth64(regs.r13); puts("  ");
	puts("R14 "); puth64(regs.r14); puts("  R15 "); puth64(regs.r15); puts("\n");

	if regs.cs != 0x08 {
		// It's a user fault, terminate that thread
		puts("Stack :");
		for i in 0 .. 4 {
			puts(" ");
			match crate::memory::user::read::<u64>(regs.rsp as usize + i * 8)
			{
			Ok(v) => puth64(v),
			Err(_) => puts("INVAL"),
			}
		}
		todo!("User fault");
	}
	else
	{
		// Kernel fault

		// For all other kernel-side errors, backtrace and lock
		let mut bp = regs.rbp;
		while let Some((newbp, ip)) = backtrace(bp)
		{
			puts(" > "); puth(ip);
			bp = newbp;
		}
		puts("\n");
	}
		
	// For interrupts 2 and 3, don't backtrace and error.
	// - 3 = breakpoint, 2 = ? (NMI?)
	if regs.intnum == 3 || regs.intnum == 2 {
		return ;
	}
	loop {}
}

fn get_cr2() -> u64
{
	// SAFE: Just reads CR2, no sideeffect
	unsafe {
		let cr2: u64;
		::core::arch::asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack, pure, preserves_flags));
		cr2
	}
}

/// Obtain the old RBP value and return address from a provided RBP value
pub fn backtrace(bp: u64) -> Option<(u64,u64)>
{
	if bp == 0 {
		return None;
	}
	if bp % 8 != 0 {
		return None;
	}
	if ! crate::memory::buf_valid(bp as *const (), 16) {
		return None;
	}
	
	// [rbp] = oldrbp, [rbp+8] = IP
	// SAFE: Pointer access checked, any alias is benign
	unsafe
	{
		let ptr: *const [u64; 2] = bp as usize as *const _;
		if ! crate::arch::memory::virt::is_reserved(ptr) {
			None
		}
		else {
			let newbp = (*ptr)[0];
			let newip = (*ptr)[1];
			// Check validity of output BP, must be > old BP (upwards on the stack)
			// - If not, return 0 (which will cause a break next loop)
			if newbp <= bp {
				Some( (0, newip) )
			}
			else {
				Some( (newbp, newip) )
			}
		}
	}
}


