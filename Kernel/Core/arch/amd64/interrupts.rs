// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/main.rs
//! Low-level interrupt handling and CPU error handling
use _common::*;
use super::{puts,puth};

#[repr(C)]
//#[derive(Copy,Clone)]
/// Register state as saved by the error ISR handler
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

#[repr(C)]
/// A handler for an ISR
pub type ISRHandler = extern "C" fn(isrnum: usize,info:*const(),idx:usize);

struct IRQHandlersEnt
{
	handler: Option<ISRHandler>,
	info: *const(),
	idx: usize,
}
impl Copy for IRQHandlersEnt {}
impl Clone for IRQHandlersEnt { fn clone(&self)->Self { *self } }
unsafe impl Send for IRQHandlersEnt {}

#[derive(Default)]
/// A handle for a bound ISR, unbound on drop
pub struct ISRHandle
{
	idx: usize,
}

#[allow(non_upper_case_globals)]
static s_irq_handlers_lock: ::sync::Mutex<[IRQHandlersEnt; 256]> = mutex_init!( [IRQHandlersEnt{
	handler: None,
	info: 0 as *const _,
	idx: 0
	}; 256] );

#[no_mangle]
#[doc(hidden)]
/// ISR handler called by assembly
pub extern "C" fn irq_handler(index: usize)
{
	let lh = s_irq_handlers_lock.lock();
	let ent = (*lh)[index];
	if let Some(h) = ent.handler {
		(h)(index, ent.info, ent.idx);
	}
}

#[no_mangle]
#[doc(hidden)]
/// Error handler called by assembly
pub extern "C" fn error_handler(regs: &InterruptRegs)
{
	puts("Error happened!\n");
	puts("Int  = "); puth(regs.intnum); puts("  Code = "); puth(regs.errorcode); puts("\n");
	puts("CS:RIP  = "); puth(regs.cs); puts(":"); puth(regs.rip); puts("\n");
	puts("SS:RSP  = "); puth(regs.ss); puts(":"); puth(regs.rsp); puts("\n");
	puts("CR2 = "); puth(get_cr2()); puts("\n");
	puts("RAX "); puth(regs.rax); puts("  RCX "); puth(regs.rcx); puts("\n");
	puts("RDX "); puth(regs.rdx); puts("  RBX "); puth(regs.rbx); puts("\n");
	// For interrupts 2 and 3, don't backtrace and error.
	// - 3 = breakpoint, 2 = ? (NMI?)
	if regs.intnum != 3 && regs.intnum != 2
	{
		let mut bp = regs.rbp;
		while let Some((newbp, ip)) = backtrace(bp)
		{
			puts(" > "); puth(ip);
			bp = newbp;
		}
		puts("\n");
		loop {}
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
	if ! ::memory::buf_valid(bp as *const (), 16) {
		return None;
	}
	
	// [rbp] = oldrbp, [rbp+8] = IP
	unsafe
	{
		let ptr: *const [u64; 2] = ::core::mem::transmute(bp);
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

#[derive(Debug,Copy,Clone)]
/// Error code for bind_isr
pub enum BindISRError
{
	Used,
}

/// Bind a callback (and params) to an allocatable ISR
pub fn bind_isr(isr: u8, callback: ISRHandler, info: *const(), idx: usize) -> Result<ISRHandle,BindISRError>
{
	log_trace!("bind_isr(isr={},callback={:?},info={:?},idx={})",
		isr, callback as *const u8, info, idx);
	// TODO: Validate if the requested ISR slot is valid (i.e. it's one of the allocatable ones)
	// 1. Check that this ISR slot on this CPU isn't taken
	let mut _mh = s_irq_handlers_lock.lock();
	let h = &mut _mh[isr as usize];
	log_trace!("&h = {:p}", h);
	if h.handler.is_some() {
		return Err( BindISRError::Used );
	}
	*h = IRQHandlersEnt {
		handler: Some(callback),
		info: info,
		idx: idx,
		};
	Ok( ISRHandle {
		idx: isr as usize,
		} )
}

impl ISRHandle
{
	/// Returns an unbound ISR handle (null)
	pub fn unbound() -> ISRHandle {
		ISRHandle {
			idx: !0,
		}
	}
	/// Returns the bound ISR index
	pub fn idx(&self) -> usize { self.idx }
}

impl ::core::ops::Drop for ISRHandle
{
	fn drop(&mut self)
	{
		if self.idx < 256
		{
			let mut mh = s_irq_handlers_lock.lock();
			let h = &mut mh[self.idx];
			h.handler = None;
		}
	}
}

// vim: ft=rust
