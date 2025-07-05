
use crate::prelude::*;
use super::super::IRQHandler;
use super::TriggerMode;
use super::APICReg;

pub struct IOAPIC
{
	regs: crate::sync::Mutex<IOAPICRegs>,
	num_lines: usize,
	first_irq: usize,
	handlers: crate::sync::Spinlock< Vec< Option<IRQHandler> > >,
}

enum Regs {
	IoapicId = 0,
	IoapicVer,
	ArbitrationId,
	RedirTable0 = 0x10,
}

struct IOAPICRegs
{
	mapping: crate::memory::virt::AllocHandle,
}

impl IOAPIC
{
	pub fn new(paddr: u64, base: usize) -> IOAPIC
	{
		let mut regs = IOAPICRegs::new(paddr);
		// First three registers are the ID, Version, and Arbitration ID
		log_debug!("regs=[{:#x},{:#x},{:#x}]",
			regs.read(Regs::IoapicId as u8),
			regs.read(Regs::IoapicVer as u8),
			regs.read(Regs::ArbitrationId as u8),
		);
		let v = regs.read(Regs::IoapicVer as _);
		let num_lines = ((v >> 16) & 0xFF) as usize + 1;
		
		log_debug!("IOAPIC #{} v{:#x}: {{ @{:#x} - GSI {} + {} }} ARB={}",
			(regs.read(Regs::IoapicId as u8) >> 24) & 0xF,
			v & 0xFF,
			paddr,
			base, num_lines,
			(regs.read(Regs::ArbitrationId as u8) >> 24) & 0xF,
		);
		IOAPIC {
			regs: crate::sync::Mutex::new( regs ),
			num_lines: num_lines,
			first_irq: base,
			handlers: crate::sync::Spinlock::new( (0..num_lines).map(|_| None).collect() ),
			}
	}
	
	pub fn contains(&self, gsi: usize) -> bool {
		self.first_irq <= gsi && gsi < self.first_irq + self.num_lines
	}
	pub fn first(&self) -> usize {
		self.first_irq
	}
	//#[is_safe(irq)]	// Holds interrupts before lock
	pub fn get_callback(&self, idx: usize) -> Option<IRQHandler> {
		assert!( idx < self.num_lines );
		let _irql = crate::sync::hold_interrupts();
		self.handlers.lock()[idx]
	}
	
	pub fn eoi(&self, _idx: usize)
	{
		// TODO: EOI in IOAPIC
		// - The LAPIC handles this
	}
	pub fn set_irq(&self, idx: usize, vector: u8, apic: u32, mode: TriggerMode, cb: IRQHandler)
	{
		log_trace!("set_irq(idx={},vector={},apic={},mode={:?})", idx, vector, apic, mode);
		assert!( idx < self.num_lines );
		let idx = idx as u8;

		// Unsynchronised write. Need to use Spinlock (with IRQ hold)?
		{
			let _irql = crate::sync::hold_interrupts();
			self.handlers.lock()[idx as usize] = Some( cb );
		}
		let flags: u32 = match mode {
			TriggerMode::EdgeHi   => (0<<13)|(0<<15),
			TriggerMode::EdgeLow  => (1<<13)|(0<<15),
			TriggerMode::LevelHi  => (0<<13)|(1<<15),
			TriggerMode::LevelLow => (1<<13)|(1<<15),
			};
		
		let mut rh = self.regs.lock();
		// Values:
		// - [63:56] Destination (4-bit LAPIC ID, or bitmask, depending on [11])
		// - [16] Mask (1 to disable interrupt)
		// - ...
		log_debug!("set_irq: (pre) Info = {:#x}", rh.read(0x10 + idx*2));
		rh.write(Regs::RedirTable0 as u8 + idx*2 + 1, (apic as u32) << 56-32 );
		rh.write(Regs::RedirTable0 as u8 + idx*2 + 0, flags | (vector as u32) );
		log_debug!("set_irq: (post) Info = {:#x} {:#x}", rh.read(0x10 + idx*2), rh.read(0x10 + idx*2 + 1));
	}
	pub fn disable_irq(&self, idx: usize)
	{
		assert!( idx < self.num_lines );
		let idx = idx as u8;
		let mut rh = self.regs.lock();
		log_debug!("disable_irq({}): Info = {:#x}", idx, rh.read(0x10 + idx*2));
		rh.write(Regs::RedirTable0 as u8 + idx*2 + 0, 1<<16);
	}

	pub fn get_irq_reg(&self, idx: usize) -> u64
	{
		assert!( idx < self.num_lines );
		let idx = idx as u8;
		let mut rh = self.regs.lock();
		
		(rh.read(Regs::RedirTable0 as u8 + idx*2 + 0) as u64) | (rh.read(Regs::RedirTable0 as u8 + idx*2 + 1) as u64) << 32
	}
}

impl IOAPICRegs
{
	fn new( paddr: u64 ) -> IOAPICRegs
	{
		// Assume SAFE: Should not end up with aliasing
		let mapping = unsafe { crate::memory::virt::map_hw_rw(paddr, 1, "IOAPIC").unwrap() };
		IOAPICRegs {
			mapping: mapping
		}
	}
	fn read(&mut self, idx: u8) -> u32
	{
		// Assume SAFE: Hardware accesses
		unsafe {
			let regs = self.mapping.as_mut::<[APICReg; 2]>(0);
			::core::intrinsics::volatile_store(&mut regs[0].data as *mut _, idx as u32);
			::core::intrinsics::volatile_load(&regs[1].data as *const _)
		}
	}
	fn write(&mut self, idx: u8, data: u32)
	{
		// Assume SAFE: Hardware accesses
		unsafe {
			let regs = self.mapping.as_mut::<[APICReg; 2]>(0);
			::core::intrinsics::volatile_store(&mut regs[0].data as *mut _, idx as u32);
			::core::intrinsics::volatile_store(&mut regs[1].data as *mut _, data)
		}
	}
	
}