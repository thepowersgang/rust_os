// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/hw/apic/raw.rs
// - x86 APIC Raw hardware API
use crate::prelude::*;

static TIMER_VEC: u8 = 0x7E;

pub struct LAPIC
{
	paddr: u64,
	mapping: crate::memory::virt::AllocHandle,
	#[allow(dead_code)]
	timer_isr: crate::arch::amd64::interrupts::ISRHandle,
}

pub struct IOAPIC
{
	regs: crate::sync::Mutex<IOAPICRegs>,
	num_lines: usize,
	first_irq: usize,
	handlers: crate::sync::Spinlock< Vec< Option<super::IRQHandler> > >,
}

struct IOAPICRegs
{
	mapping: crate::memory::virt::AllocHandle,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum TriggerMode
{
	LevelHi,
	LevelLow,
	EdgeHi,
	EdgeLow,
}

#[allow(dead_code)]
#[repr(u8)]
#[derive(Copy,Clone)]
#[allow(non_camel_case_types)]
enum ApicReg
{
	LAPIC_ID  = 0x2,
	LAPIC_Ver = 0x3,
	TPR       = 0x8,	// Task Priority
	APR       = 0x9,	// Arbitration Priority
	PPR       = 0xA,	// Processor Priority
	EOI       = 0xB,
	RRD       = 0xC,	// Remote Read
	LocalDest = 0xD,	// Local Destination
	DestFmt   = 0xE,	// Destination Format
	SIR       = 0xF,	// Spurious Interrupt Vector
	InService = 0x10,	// In-Service Register (First of 8)
	TMR       = 0x18,	// Trigger Mode (1/8)
	IRR       = 0x20,	// Interrupt Request Register (1/8)
	ErrStatus = 0x28,	// Error Status
	LVTCMCI   = 0x2F,	// LVT CMCI Registers (?)
	ICR       = 0x30,	// Interrupt Command Register (1/2)
	LVTTimer  = 0x32,
	LVTThermalSensor = 0x33,
	LVTPermCounters  = 0x34,
	LVT_LINT0 = 0x35,
	LVT_LINT1 = 0x36,
	LVT_Error = 0x37,
	InitCount = 0x38,
	CurCount  = 0x39,
	TmrDivide = 0x3E,
}

#[repr(C)]
struct APICReg
{
	data: u32,
	_rsvd: [u32; 3],
}

impl LAPIC
{
	pub fn new(paddr: u64) -> LAPIC
	{
		let ret = LAPIC {
			paddr: paddr,
			// Assume SAFE: Shouldn't be aliasing
			mapping: unsafe { crate::memory::virt::map_hw_rw(paddr, 1, "APIC").unwrap() },
			timer_isr: Default::default(),
			};
		
		log_debug!("LAPIC {{ IDReg={:x}, Ver={:x}, SIR={:#x} }}",
			ret.read_reg(ApicReg::LAPIC_ID),
			ret.read_reg(ApicReg::LAPIC_Ver),
			ret.read_reg(ApicReg::SIR)
			);
		
		ret
	}
	/// Initialise the LAPIC structures once self is in its final location
	pub fn global_init(&mut self)
	{
		self.timer_isr = match crate::arch::amd64::interrupts::bind_isr(TIMER_VEC, lapic_timer, self as *mut _ as *const (), 0)
			{
			Ok(v) => v,
			Err(e) => panic!("Unable to bind LAPIC timer: {:?}", e),
			};
	}
	/// Initialise the LAPIC (for this CPU)
	pub fn init(&self)
	{
		// SAFE: Read original LAPIC base
		let oldaddr = unsafe{
			let a: u32;
			let d: u32;
			::core::arch::asm!("rdmsr", lateout("eax") a, lateout("edx") d, in("ecx") 0x1Bu32, options(pure, readonly));
			(d as u64) << 32 | a as u64
			};
		log_debug!("oldaddr = {:#x}", oldaddr);
		let is_bsp = oldaddr & 0x100;
		log_debug!("IRR[0..8] = [{:#x}, {:#x}, {:#x}, {:#x},  {:#x}, {:#x}, {:#x}, {:#x}]",
			self.read_reg(ApicReg::irr(0)),
			self.read_reg(ApicReg::irr(1)),
			self.read_reg(ApicReg::irr(2)),
			self.read_reg(ApicReg::irr(3)),
			self.read_reg(ApicReg::irr(4)),
			self.read_reg(ApicReg::irr(5)),
			self.read_reg(ApicReg::irr(6)),
			self.read_reg(ApicReg::irr(7))
			);
		
		//self.write_reg(ApicReg::SIR as usize, self.read_reg(ApicReg_SIR as usize) | (1 << 8));
		self.write_reg(ApicReg::SIR, 0x7F | (1 << 8));	// Enable LAPIC (and set Spurious to 127)
		self.write_reg(ApicReg::InitCount, 0x100000);
		self.write_reg(ApicReg::TmrDivide, 3);	// Timer Divide = 16
		self.write_reg(ApicReg::LVTTimer, TIMER_VEC as u32);	// Enable Timer
		self.write_reg(ApicReg::LVTThermalSensor, 0);	// "Disable" Thermal Sensor
		self.write_reg(ApicReg::LVTPermCounters, 0);	// "Disable" ? Counters
		self.write_reg(ApicReg::LVT_LINT0, 0);	// "Disable" LINT0
		self.write_reg(ApicReg::LVT_LINT1, 0);	// "Disable" LINT1
		self.write_reg(ApicReg::LVT_Error, 0);	// "Disable" Error
		// EOI - Just to make sure
		self.eoi(0);
		// SAFE: Write MSR, values should be correct
		unsafe {
			::core::arch::asm!("wrmsr", in("ecx") 0x1Bu32, in("edx") self.paddr >> 32, in("eax") (self.paddr | is_bsp | 0x800), options(nomem));
		}
	}
	//#[is_safe(irq)]
	pub fn eoi(&self, num: usize)
	{
		self.write_reg(ApicReg::EOI, num as u32);
	}
	
	fn read_reg(&self, reg: ApicReg) -> u32
	{
		// SAFE: Aligned memory accesses to hardware are atomic on x86
		unsafe {
			let regs = self.mapping.as_ref::<[APICReg; 64]>(0);
			assert!( (reg as usize) < 64 );
			::core::intrinsics::volatile_load( &regs[reg as usize].data as *const _ )
		}
	}
	fn write_reg(&self, idx: ApicReg, value: u32)
	{
		// SAFE: Aligned memory accesses to hardware are atomic on x86
		unsafe {
			let regs = self.mapping.as_int_mut::<[APICReg; 64]>(0);
			assert!( (idx as usize) < 64 );
			::core::intrinsics::volatile_store( &mut regs[idx as usize].data as *mut _, value )
		}
	}
	
	pub fn get_vec_status(&self, idx: u8) -> (bool,bool,bool, u32)
	{
		let reg = idx / 32;
		let bit = idx % 32;
		let mask = 1 << bit;
		let in_svc = self.read_reg(ApicReg::in_service(reg)) & mask != 0;
		let mode   = self.read_reg(ApicReg::tmr(reg)) & mask != 0;
		let in_req = self.read_reg(ApicReg::irr(reg)) & mask != 0;
		let err = self.read_reg(ApicReg::ErrStatus);
		
		(in_svc, mode, in_req, err)
	}
	
	fn local_timer(isr: usize, sp: *const (), _idx: usize)
	{
		assert!( !sp.is_null() );
		// SAFE: 'sp' is the bound pointer, and should be valid
		let s: &LAPIC = unsafe { &*(sp as *const LAPIC) };
		log_trace!("LAPIC Timer");
		s.eoi(isr);
	}
}
impl ApicReg
{
	fn in_service(reg: u8) -> ApicReg
	{
		assert!(reg < 8);
		// SAFE: Transmutes to a u8 repr enum with a valid value
		unsafe { ::core::mem::transmute(ApicReg::InService as u8 + reg as u8) }
	}
	fn tmr(reg: u8) -> ApicReg
	{
		assert!(reg < 8);
		// SAFE: Transmutes to a u8 repr enum with a valid value
		unsafe { ::core::mem::transmute(ApicReg::TMR as u8 + reg as u8) }
	}
	fn irr(reg: u8) -> ApicReg
	{
		assert!(reg < 8);
		// SAFE: Transmutes to a u8 repr enum with a valid value
		unsafe { ::core::mem::transmute(ApicReg::IRR as u8 + reg as u8) }
	}
}

extern "C" fn lapic_timer(isr: usize, sp: *const (), _idx: usize)
{
	LAPIC::local_timer(isr, sp, _idx);	
}

impl IOAPIC
{
	pub fn new(paddr: u64, base: usize) -> IOAPIC
	{
		let mut regs = IOAPICRegs::new(paddr);
		let v = regs.read(1);
		log_debug!("{:x} {:x} {:x}", v, v>>16, (v >> 16) & 0xFF);
		let num_lines = ((v >> 16) & 0xFF) as usize + 1;
		log_debug!("regs=[{:#x},{:#x},{:#x}]", regs.read(0), regs.read(1), regs.read(2));
		
		log_debug!("IOAPIC: {{ {:#x} - {} + {} }}", paddr, base, num_lines);
		IOAPIC {
			regs: crate::sync::Mutex::new( regs ),
			num_lines: num_lines,
			first_irq: base,
			handlers: crate::sync::Spinlock::new( Vec::from_fn(num_lines, |_| None) ),
			}
	}
	
	pub fn contains(&self, gsi: usize) -> bool {
		self.first_irq <= gsi && gsi < self.first_irq + self.num_lines
	}
	pub fn first(&self) -> usize {
		self.first_irq
	}
	//#[is_safe(irq)]	// Holds interrupts before lock
	pub fn get_callback(&self, idx: usize) -> Option<super::IRQHandler> {
		assert!( idx < self.num_lines );
		let _irql = crate::sync::hold_interrupts();
		self.handlers.lock()[idx]
	}
	
	pub fn eoi(&self, _idx: usize)
	{
		// TODO: EOI in IOAPIC
	}
	pub fn set_irq(&self, idx: usize, vector: u8, apic: u32, mode: TriggerMode, cb: super::IRQHandler)
	{
		log_trace!("set_irq(idx={},vector={},apic={},mode={:?})", idx, vector, apic, mode);
		assert!( idx < self.num_lines );

		// Unsynchronised write. Need to use Spinlock (with IRQ hold)?
		{
			let _irql = crate::sync::hold_interrupts();
			self.handlers.lock()[idx] = Some( cb );
		}
		let flags: u32 = match mode {
			TriggerMode::EdgeHi   => (0<<13)|(0<<15),
			TriggerMode::EdgeLow  => (1<<13)|(0<<15),
			TriggerMode::LevelHi  => (0<<13)|(1<<15),
			TriggerMode::LevelLow => (1<<13)|(1<<15),
			};
		
		let mut rh = self.regs.lock();
		log_debug!("Info = {:#x}", rh.read(0x10 + idx*2));
		rh.write(0x10 + idx*2 + 1, (apic as u32) << 56-32 );
		rh.write(0x10 + idx*2 + 0, flags | (vector as u32) );
	}
	pub fn disable_irq(&self, idx: usize)
	{
		let mut rh = self.regs.lock();
		log_debug!("Disable {}: Info = {:#x}", idx, rh.read(0x10 + idx*2));
		rh.write(0x10 + idx*2 + 0, 1<<16);
	}

	pub fn get_irq_reg(&self, idx: usize) -> u64
	{
		let mut rh = self.regs.lock();
		
		(rh.read(0x10 + idx*2 + 0) as u64) | (rh.read(0x10 + idx*2 + 1) as u64) << 32
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
	fn read(&mut self, idx: usize) -> u32
	{
		// Assume SAFE: Hardware accesses
		unsafe {
			let regs = self.mapping.as_mut::<[APICReg; 2]>(0);
			::core::intrinsics::volatile_store(&mut regs[0].data as *mut _, idx as u32);
			::core::intrinsics::volatile_load(&regs[1].data as *const _)
		}
	}
	fn write(&mut self, idx: usize, data: u32)
	{
		// Assume SAFE: Hardware accesses
		unsafe {
			let regs = self.mapping.as_mut::<[APICReg; 2]>(0);
			::core::intrinsics::volatile_store(&mut regs[0].data as *mut _, idx as u32);
			::core::intrinsics::volatile_store(&mut regs[1].data as *mut _, data)
		}
	}
	
}

// vim: ft=rust
