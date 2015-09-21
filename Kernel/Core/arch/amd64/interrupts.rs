// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/main.rs
//! Low-level interrupt handling and CPU error handling
//use prelude::*;

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

#[derive(Debug,Copy,Clone)]
/// Error code for bind_isr
pub enum BindISRError
{
	Used,
}

pub use super::hw::apic::IRQHandle;
pub use super::hw::apic::IrqError as BindError;
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
