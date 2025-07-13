
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
		for i in 0 .. num_lines as u8 {
			let reg = Regs::RedirTable0 as u8 + i*2;
			log_debug!("IRQ {:2} = 0x{:8x} {:8x}", base + i as usize, regs.read(reg + 1), regs.read(reg + 0));
			let v = regs.read(reg + 0) | 1 << 16;
			regs.write(reg + 0, v);
		}
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

		// Unsynchronised write. Need to use Spinlock (with IRQ hold)?
		{
			let _irql = crate::sync::hold_interrupts();
			self.handlers.lock()[idx] = Some( cb );
		}
		let flags = match mode {
			TriggerMode::EdgeHi   => (0<<13)|(0<<15),
			TriggerMode::EdgeLow  => (1<<13)|(0<<15),
			TriggerMode::LevelHi  => (0<<13)|(1<<15),
			TriggerMode::LevelLow => (1<<13)|(1<<15),
			};
		
		// Values:
		// - [63:56] Destination (4-bit LAPIC ID, or bitmask, depending on [11])
		// - [16] Mask (1 to disable interrupt)
		// - ...
		self.set_irq_reg(idx, (apic as u64) << 56 | flags | vector as u64);
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
	fn set_irq_reg(&self, idx: usize, new_v: u64)
	{
		let mut rh = self.regs.lock();

		let reg = Regs::RedirTable0 as u8 + idx as u8 * 2;
		log_debug!("set_irq_reg: (pre) Info = 0x {:08x} {:08x}", rh.read(reg+1), rh.read(reg));

		let valid_mask = 0xFF000000_0001FFFF;
		let v = rh.read_pair(reg);
		rh.write_pair(reg, v & !valid_mask | new_v);

		log_debug!("set_irq: (post) Info = 0x {:08x} {:08x}", rh.read(reg+1), rh.read(reg));
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
	fn read(&mut self, reg: u8) -> u32
	{
		// Assume SAFE: Hardware accesses
		unsafe {
			let regs = self.mapping.as_mut::<[APICReg; 2]>(0);
			::core::intrinsics::volatile_store(&mut regs[0].data as *mut _, reg as u32);
			::core::intrinsics::volatile_load(&regs[1].data as *const _)
		}
	}
	fn write(&mut self, reg: u8, data: u32)
	{
		// Assume SAFE: Hardware accesses
		unsafe {
			let regs = self.mapping.as_mut::<[APICReg; 2]>(0);
			::core::intrinsics::volatile_store(&mut regs[0].data as *mut _, reg as u32);
			::core::intrinsics::volatile_store(&mut regs[1].data as *mut _, data)
		}
	}
	
	fn read_pair(&mut self, reg: u8) -> u64 {
		let v1 = self.read(reg + 0);
		let v2 = self.read(reg + 1);
		v1 as u64 | (v2 as u64) << 32
	}
	fn write_pair(&mut self, reg: u8, val: u64) {
		self.write(reg+1, (val >> 32) as u32);
		self.write(reg, val as u32);
	}
}