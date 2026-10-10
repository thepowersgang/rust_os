//! Local APIC - See Intel Manuals, 3A Ch 10 ("Advanced Programmable Interrupt Controller")
// cspell:ignore S_IOAPICS
use super::ApicReg;
use super::APICReg;

const ISR_SPURIOUS: u8 = 0x7F;
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
		let old_addr = unsafe{
			let a: u32;
			let d: u32;
			::core::arch::asm!("rdmsr", lateout("eax") a, lateout("edx") d, in("ecx") 0x1Bu32, options(pure, readonly));
			(d as u64) << 32 | a as u64
			};
		log_debug!("old_addr = {:#x}", old_addr);
		let is_bsp = old_addr & 0x100;
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
		
		// Get frequency using CPUID 0x15 and 0x16
		// - An internal LAPIC (most modern hardware) will use the core frequency, externals the bus freq
		// - See https://sandpile.org/x86/cpuid.htm#leaf_0000_0015h
		// Clear MISC_ENABLE.LCMV (Limit CPUID Max Val) first, as the CPU doesn't expose CPUID 0x15 unless that is set (to mitigate a WinNT bug)
		// SAFE: This MSR has no memory effects.
		unsafe {
			::core::arch::asm!( "rdmsr; and eax, ~(1 << 22); wrmsr", in("rcx") 0x1A0, out("rax") _, out("rdx") _ );
		}
		let bus_mhz = ::core::arch::x86_64::__cpuid(0x16).ecx & 0xFFFF;
		let core_hz = ::core::arch::x86_64::__cpuid(0x15).ecx;
		let freq_mhz = match (bus_mhz, core_hz) {
			(0,0) => 1_000,	// Qemu
			(100,0) => 100,	// Dell Mini-PC (guessed) - 1000MHz caused the timer to fire after (72.4s-30s=)42s
			_ => core_hz / 1_000_000,
		};
		log_debug!("Frequencies: {} Hz (core) or {} MHz (bus) (picked {freq_mhz} MHz)",
			::core::arch::x86_64::__cpuid(0x15).ecx,
			::core::arch::x86_64::__cpuid(0x16).ecx & 0xFFFF,
		);

		//self.write_reg(ApicReg::SIR as usize, self.read_reg(ApicReg_SIR as usize) | (1 << 8));
		self.write_reg(ApicReg::SIR, ISR_SPURIOUS as u32 | (1 << 8));	// Enable LAPIC (and set Spurious Vector to 127)
		self.write_reg(ApicReg::InitCount, !0);	// Set the count to max value, just so it doesn't trigger early
		self.write_reg(ApicReg::TmrDivide, 0b1010);	// Divide by 128 (0b110 in bits 3,1,0 - bit 2 is reserved 0)
		self.write_reg(ApicReg::LVTTimer, (0b01 << 17)|(0 << 16)|(ISR_LAPIC_TIMER as u32));	// Periodic, Unmasked, bind to vector 126
		// Set the LAPIC timer to 1Hz, i.e. set to the frequency
		self.write_reg(ApicReg::InitCount, freq_mhz * (1_000_000 / 128));	// Set count timer after everything

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
		self.write_reg(ApicReg::EOI, 0);	// Apparently zero is the only valid value?
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
	
	/// LAPIC Timer handler
	fn local_timer(isr: usize, sp: *const (), _idx: usize)
	{
		assert!( !sp.is_null() );
		// SAFE: 'sp' is the bound pointer, and should be valid
		let s: &LAPIC = unsafe { &*(sp as *const LAPIC) };
		s.eoi(isr);
		if true {
			return 
		}
		if true
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
				static mut SAVED_REGS: [u64; 256] = [1<<16; 256];
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
				let iir = inb(PORT_BASE+2);
				log_debug!("RS232 IIR={:02x} (FS={iir_fs}, ST={iir_st},nP={iir_np}), LSR={lsr:02x}",
					iir,
					lsr=inb(PORT_BASE+5),
					iir_fs = ["None","Unusable","Enabled","UD3"][ ((iir >> 6) & 3) as usize ],	// FIFO state
					iir_st = ["ModemSts","TxEmpty","RxAvail","RxLineSts",][ ((iir >> 1) & 3) as usize ],// interrupt state
					iir_np = (iir & 1),	// Not pending
					);
				while inb(PORT_BASE+5) & 1 == 1 {
					let c = inb(PORT_BASE);
					log_debug!("RS232 Rx = 0x{:02x} ({:?})", c, c as char)
				}
			}
		}
		
	}
}
