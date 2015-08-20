// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/main.rs
//! Low-level interrupt handling and CPU error handling
use prelude::*;
use super::{puts,puth};
use super::log::puth64;

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

static S_IRQ_HANDLERS_LOCK: ::sync::Spinlock<[IRQHandlersEnt; 256]> = ::sync::Spinlock::new( [IRQHandlersEnt{
	handler: None,
	info: 0 as *const _,
	idx: 0
	}; 256] );

#[no_mangle]
#[doc(hidden)]
#[tag_safe(irq)]
/// ISR handler called by assembly
pub extern "C" fn irq_handler(index: usize)
{
	let lh = S_IRQ_HANDLERS_LOCK.lock_irqsafe();
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
	match regs.intnum
	{
	13 => { puts("GPF ("); puth(regs.errorcode); puts(")\n"); },
	14 => {
		let cr2;
		// SAFE: Just reads CR2
		unsafe { asm!("mov %cr2, $0" : "=r" (cr2)) };
		puts("PF ("); puth(regs.errorcode); puts(") at "); puth(cr2 as u64); puts(" by "); puth(regs.rip); puts(" SP="); puth(regs.rsp); puts("\n");
		if ::arch::memory::virt::handle_page_fault(cr2, regs.errorcode as u32) {
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
	// SAFE: Just reads CR2, no sideeffect
	unsafe {
		let cr2: u64;
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

pub use super::hw::apic::IRQHandle;
pub use super::hw::apic::register_irq as bind_gsi;

/// Bind a callback (and params) to an allocatable ISR
pub fn bind_isr(isr: u8, callback: ISRHandler, info: *const(), idx: usize) -> Result<ISRHandle,BindISRError>
{
	log_trace!("bind_isr(isr={},callback={:?},info={:?},idx={})",
		isr, callback as *const u8, info, idx);
	// TODO: Validate if the requested ISR slot is valid (i.e. it's one of the allocatable ones)
	// 1. Check that this ISR slot on this CPU isn't taken
	let _irq_hold = ::arch::sync::hold_interrupts();
	let mut mh = S_IRQ_HANDLERS_LOCK.lock();
	let h = &mut mh[isr as usize];
	log_trace!("&h = {:p}", h);
	if h.handler.is_some()
	{
		Err( BindISRError::Used )
	}
	else
	{
		// 2. Assign
		*h = IRQHandlersEnt {
			handler: Some(callback),
			info: info,
			idx: idx,
			};
		Ok( ISRHandle {
			idx: isr as usize,
			} )
	}
}

pub fn bind_free_isr(callback: ISRHandler, info: *const(), idx: usize) -> Result<ISRHandle,BindISRError>
{
	log_trace!("bind_free_isr(callback={:?},info={:?},idx={})", callback as *const u8, info, idx);
	
	let _irq_hold = ::arch::sync::hold_interrupts();
	let mut lh = S_IRQ_HANDLERS_LOCK.lock();
	for i in 32 .. lh.len()
	{
		if lh[i].handler.is_none() {
			log_trace!("- Using ISR {}", i);
			lh[i] = IRQHandlersEnt {
				handler: Some(callback),
				info: info,
				idx: idx,
				};
			return Ok( ISRHandle {
				idx: i,
				} )
		}
	}
	
	Err( BindISRError::Used )
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
			let _irq_hold = ::arch::sync::hold_interrupts();
			let mut mh = S_IRQ_HANDLERS_LOCK.lock();
			let h = &mut mh[self.idx];
			h.handler = None;
		}
	}
}

// vim: ft=rust
