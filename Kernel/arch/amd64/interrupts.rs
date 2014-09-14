//
//
//
use super::{puts,puth};

#[repr(C)]
struct InterruptRegs
{
	fs: u64,
	gs: u64,

	rax: u64, rcx: u64, rdx: u64, rbx: u64,
	kernelrsp: u64, rbp: u64, rsi: u64, rdi: u64,
	r8: u64,  r9: u64,  r10: u64, r11: u64,
	r12: u64, r13: u64, r14: u64, r15: u64,
	
	intnum: u64, errorcode: u64,
	rip: u64, cs: u64,
	rflags: u64, rsp: u64, ss: u64,
}

#[no_mangle]
pub extern "C" fn error_handler(regs: &InterruptRegs)
{
	puts("Error happened!\n");
	puts("Int  = "); puth(regs.intnum as uint); puts("\n");
	puts("Code = "); puth(regs.errorcode as uint); puts("\n");
	puts("CS:RIP  = "); puth(regs.cs as uint); puts(":"); puth(regs.rip as uint); puts("\n");
	puts("SS:RSP  = "); puth(regs.ss as uint); puts(":"); puth(regs.rsp as uint); puts("\n");
	loop {}	
}

// vim: ft=rust
