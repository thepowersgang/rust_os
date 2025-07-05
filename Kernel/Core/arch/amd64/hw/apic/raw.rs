// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/hw/apic/raw.rs
// - x86 APIC Raw hardware API

mod lapic;
pub use self::lapic::LAPIC;
mod ioapic;
pub use self::ioapic::IOAPIC;

#[allow(dead_code)]
#[derive(Debug)]
pub enum TriggerMode
{
	LevelHi,
	LevelLow,
	EdgeHi,
	EdgeLow,
}

#[repr(u8)]
#[allow(dead_code)]
pub enum DeliveryMode {
	Normal = 0,
	LowPriority = 1,
	SystemManagementInterrupt = 2,
	NonMaskableInterrupt = 4,
	InitIPI = 5,
	StartupIPI = 6,
	External = 7,
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
	Icr0      = 0x30,	// Interrupt Command Register (1/2)
	Icr1      = 0x31,	// Interrupt Command Register (2/2)
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

// vim: ft=rust
