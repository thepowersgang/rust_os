//! Local APIC - See Intel Manuals, 3A Ch 10 ("Advanced Programmable Interrupt Controller")
use super::ApicReg;
use super::APICReg;

const ISR_SPURRIOUS: u8 = 0x7F;
const ISR_LAPIC_TIMER: u8 = 0x7E;

pub struct LAPIC
{
	paddr: u64,
	mapping: crate::memory::virt::AllocHandle,
	#[allow(dead_code)]
	timer_isr: crate::arch::amd64::interrupts::ISRHandle,
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
		extern "C" fn lapic_timer(isr: usize, sp: *const (), _idx: usize)
		{
			LAPIC::local_timer(isr, sp, _idx);	
		}
		self.timer_isr = match crate::arch::amd64::interrupts::bind_isr(ISR_LAPIC_TIMER, lapic_timer, self as *mut _ as *const (), 0)
			{
			Ok(v) => v,
			Err(e) => panic!("Unable to bind LAPIC timer: {:?}", e),
			};
	}
	/// Initialise the LAPIC (for this CPU)
	pub fn percpu_init(&self, allow_pic: bool)
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
		log_debug!("IRR[0..8] = [{:08x}, {:08x}, {:08x}, {:08x},  {:08x}, {:08x}, {:08x}, {:08x}]",
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
		self.write_reg(ApicReg::SIR, ISR_SPURRIOUS as u32 | (1 << 8));	// Enable LAPIC (and set Spurious Vector to 127)
		self.write_reg(ApicReg::InitCount, 0x1000000);	// ~16M
		self.write_reg(ApicReg::TmrDivide, 3);	// Timer Divide = 16
		self.write_reg(ApicReg::LVTTimer, (0b01 << 17)|(0 << 16)|(ISR_LAPIC_TIMER as u32));	// Periodic, Unmasked, bind to vector 126
		self.write_reg(ApicReg::LVTThermalSensor, 0x10000);	// "Disable" Thermal Sensor
		self.write_reg(ApicReg::LVTPermCounters, 0x10000);	// "Disable" ? Counters
		if !allow_pic {
			self.write_reg(ApicReg::LVT_LINT0, 0x10000);	// "Disable" LINT0
		}
		else {
			self.write_reg(ApicReg::LVT_LINT0, 0x007FF);	// Enable Local Interrupt 0, delivery mode [8:10]=7 External, vector=255
		}
		self.write_reg(ApicReg::LVT_LINT1, 0x10000);	// "Disable" LINT1
		self.write_reg(ApicReg::LVT_Error, 0x10000);	// "Disable" Error
		// EOI - Just to make sure
		self.eoi(0);
		// SAFE: Write MSR, values should be correct
		unsafe {
			::core::arch::asm!("wrmsr",
				in("ecx") 0x1Bu32,
				in("edx") self.paddr >> 32, in("eax") (self.paddr | is_bsp | 0x800),
				options(nomem)
			);
		}
	}
	//#[is_safe(irq)]
	pub fn eoi(&self, _num: usize)
	{
		//self.write_reg(ApicReg::EOI, num as u32);
		self.write_reg(ApicReg::EOI, 0);	// Aparently zero is the only valid value?
	}
	// UNSAFE: IPIs can request ACE (via StartupIPI)
	pub unsafe fn send_ipi(&self, apic_id: u8, vector: u8, delivery_mode: super::DeliveryMode) {
		self.write_reg(ApicReg::Icr1, (apic_id as u32) << 24);
		self.write_reg(ApicReg::Icr0, (vector as u32) | ((delivery_mode as u32) << 8) | 0xC000);
		// Wait until listed as delivered
		while self.read_reg(ApicReg::Icr0) & (1 << 12) != 0 {
			::core::hint::spin_loop();
		}
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
		s.eoi(isr);
		if true {
			return 
		}
		if false
		{
			log_trace!("LAPIC Timer");
			log_debug!("IRR[0..8] = [{:#x}, {:#x}, {:#x}, {:#x},  {:#x}, {:#x}, {:#x}, {:#x}]",
				s.read_reg(ApicReg::irr(0)),
				s.read_reg(ApicReg::irr(1)),
				s.read_reg(ApicReg::irr(2)),
				s.read_reg(ApicReg::irr(3)),
				s.read_reg(ApicReg::irr(4)),
				s.read_reg(ApicReg::irr(5)),
				s.read_reg(ApicReg::irr(6)),
				s.read_reg(ApicReg::irr(7))
				);
			log_debug!("ISR[0..8] = [{:#x}, {:#x}, {:#x}, {:#x},  {:#x}, {:#x}, {:#x}, {:#x}]",
				s.read_reg(ApicReg::in_service(0)),
				s.read_reg(ApicReg::in_service(1)),
				s.read_reg(ApicReg::in_service(2)),
				s.read_reg(ApicReg::in_service(3)),
				s.read_reg(ApicReg::in_service(4)),
				s.read_reg(ApicReg::in_service(5)),
				s.read_reg(ApicReg::in_service(6)),
				s.read_reg(ApicReg::in_service(7))
				);
		}

		if crate::arch::cpu_num() == 0 {
			if super::super::S_IOAPICS.is_empty() {
				use super::super::super::pic;
				let status = pic::read_status();
				// SAFE: Single-threaded code
				let prev = unsafe {
					static mut SAVED_REGS: [pic::Status; 2] = [pic::Status { isr: 0, irr: 0}; 2];
					let prev = SAVED_REGS;
					SAVED_REGS = status;
					prev
				};
				if prev != status {
					log_debug!("PIC Was = {:x?},{:x?}", prev[0], prev[1]);
					log_debug!("PIC Was = {:x?},{:x?}", status[0], status[1]);
				}
			}
			else {
				static mut SAVED_REGS: [u64; 256] = [0; 256];
				//log_debug!("IOAPIC IRQ4 = {:#x}", super::super::S_IOAPICS[0].get_irq_reg(4));
				for i in 0..super::super::S_IOAPICS[0].num_lines() {
					let v = super::super::S_IOAPICS[0].get_irq_reg(i);
					// SAFE: Static mut access just reads/writes - no overlapping mutable access (this is CPU0 only)
					let prev = unsafe {
						if v == SAVED_REGS[i] {
							continue ;
						}
						let prev = SAVED_REGS[i];
						SAVED_REGS[i] = v;
						prev
					};
					log_debug!("IOAPIC IRQ{:3} = 0x{:16x} {:?}", i, v, super::ioapic::DumpRedir(v) );
					log_debug!("IOAPIC Was    = 0x{:16x} {:?}", prev, super::ioapic::DumpRedir(prev));
				}
			}
			// SAFE: Correct access to serial port
			unsafe {
				const PORT_BASE: u16 = 0x3F8;
				use crate::arch::amd64::x86_io::inb;
				log_debug!("RS232 IIR={:02x}, LSR={:02x}",
					inb(PORT_BASE+2),
					inb(PORT_BASE+5),
					);
			}
		}
		
	}
}
