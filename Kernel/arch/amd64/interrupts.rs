//
//
//
use core::ptr::RawPtr;
use core::result::{Result,Ok,Err};
use super::{puts,puth};

#[repr(C)]
pub struct InterruptRegs
{
	fs: u64,
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
struct IRQHandlersEnt
{
	bound: bool,
	handler: fn(*const()),
	info: *const(),
	cleanup: extern "C" fn(uint, bool),
}

#[deriving(Default)]
pub struct ISRHandle
{
	idx: uint,
}

static mut s_irq_handlers_lock: ::sync::Mutex<()> = mutex_init!( () );
extern "C"
{
	static mut IrqHandlers: [IRQHandlersEnt,..256];
}

#[no_mangle]
#[allow(visible_private_types)]
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
		loop {}
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


pub fn bind_isr(cpu_num: int, isr: u8, callback: fn (*const ()), info: *const(), cleanup: extern "C" fn(uint,bool)) -> Result<ISRHandle,()>
{
	log_trace!("bind_isr(cpu_num={},isr={},callback={},info={})", cpu_num, isr, callback as *const u8, info);
	// 1. Check that this ISR slot on this CPU isn't taken
	let _mh = unsafe { s_irq_handlers_lock.lock() };
	let h = unsafe { &mut IrqHandlers[isr as uint] };
	log_trace!("&h = {}", h as *mut _);
	if h.bound {
		return Err( () );
	}
	h.bound = true;
	// 2. Create a new ISR on the heap, populated with the callback and info ptr
	// 3. And assign that to the ISR slot
	h.handler = callback;
	h.info = info;
	h.cleanup = cleanup;
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
		let _mh = unsafe { s_irq_handlers_lock.lock() };
		let h = unsafe { &mut IrqHandlers[self.idx] };
		h.bound = false;
	}
}

// vim: ft=rust
