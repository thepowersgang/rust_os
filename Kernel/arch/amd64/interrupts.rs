//
//
//
use _common::*;
use super::{puts,puth};

#[repr(C)]
pub struct InterruptRegs
{
	//fs: u64,
	gs: u64,

	rax: u64, rcx: u64, rdx: u64, rbx: u64,
	/*no rsp*/rbp: u64, rsi: u64, rdi: u64,
	r8: u64,  r9: u64,  r10: u64, r11: u64,
	r12: u64, r13: u64, r14: u64, r15: u64,
	
	intnum: u64, errorcode: u64,
	rip: u64, cs: u64,
	rflags: u64, rsp: u64, ss: u64,
}

pub type ISRHandler = extern "C" fn(isrnum: uint,info:*const(),idx:uint);

#[repr(C)]
struct IRQHandlersEnt
{
	bound: bool,
	// 
	handler: ISRHandler,
	info: *const(),
	idx: uint,
}

#[deriving(Default)]
pub struct ISRHandle
{
	idx: uint,
}

#[allow(non_upper_case_globals)]
static s_irq_handlers_lock: ::sync::Mutex<()> = mutex_init!( () );
extern "C"
{
	static mut IrqHandlers: [IRQHandlersEnt,..256];
}

#[no_mangle]
pub extern "C" fn error_handler(regs: &InterruptRegs)
{
	puts("Error happened!\n");
	puts("Int  = "); puth(regs.intnum as uint); puts("  Code = "); puth(regs.errorcode as uint); puts("\n");
	puts("CS:RIP  = "); puth(regs.cs as uint); puts(":"); puth(regs.rip as uint); puts("\n");
	puts("SS:RSP  = "); puth(regs.ss as uint); puts(":"); puth(regs.rsp as uint); puts("\n");
	puts("CR2 = "); puth(get_cr2() as uint); puts("\n");
	puts("RAX "); puth(regs.rax as uint); puts("  RCX "); puth(regs.rcx as uint); puts("\n");
	puts("RDX "); puth(regs.rdx as uint); puts("  RBX "); puth(regs.rbx as uint); puts("\n");
	if regs.intnum != 3
	{
		let mut bp = regs.rbp;
		while let Some((newbp, ip)) = backtrace(bp)
		{
			puts(" > "); puth(ip as uint);
			bp = newbp;
		}
		puts("\n");
		loop {}
	}
}

fn backtrace(bp: u64) -> Option<(u64,u64)>
{
	if bp == 0 {
		return None;
	}
	if bp % 8 != 0 {
		return None;
	}
	if ! ::memory::buf_valid(bp as *const (), 16) {
		return None;
	}
	
	// [rbp] = oldrbp, [rbp+8] = IP
	unsafe
	{
		let ptr: *const [u64,..2] = ::core::mem::transmute(bp);
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

fn get_cr2() -> u64
{
	unsafe {
		let mut cr2: u64;
		asm!("movq %cr2, $0" : "=r" (cr2));
		cr2
	}
}

/// Bind a callback (and params) to an allocatable ISR
pub fn bind_isr(isr: u8, callback: ISRHandler, info: *const(), idx: uint) -> Result<ISRHandle,()>
{
	log_trace!("bind_isr(isr={},callback={},info={},idx={})",
		isr, callback as *const u8, info, idx);
	// TODO: Validate if the requested ISR slot is valid (i.e. it's one of the allocatable ones)
	// 1. Check that this ISR slot on this CPU isn't taken
	let _mh = s_irq_handlers_lock.lock();
	let h = unsafe { &mut IrqHandlers[isr as uint] };
	log_trace!("&h = {}", h as *mut _);
	if h.bound {
		return Err( () );
	}
	*h = IRQHandlersEnt {
		bound: true,
		handler: callback,
		info: info,
		idx: idx,
		};
	Ok( ISRHandle {
		idx: isr as uint,
		} )
}

impl ISRHandle
{
	pub fn idx(&self) -> uint { self.idx }
}

impl ::core::ops::Drop for ISRHandle
{
	fn drop(&mut self)
	{
		let _mh = s_irq_handlers_lock.lock();
		let h = unsafe { &mut IrqHandlers[self.idx] };
		h.bound = false;
	}
}

// vim: ft=rust
