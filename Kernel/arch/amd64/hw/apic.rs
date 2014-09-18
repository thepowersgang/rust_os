// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/hw/apic.rs
// - x86 APIC (Advanced Programmable Interrupt Controller) driver
use _common::*;

module_define!(APIC, [], init)

#[repr(C)]
struct APICReg
{
	data: u32,
	_rsvd: [u32,..3],
}

pub struct APIC
{
	regs: &'static mut [APICReg, ..4096/16],
}

extern "C" {
	static mut s_lapic_mapping: [APICReg, ..4096/16];
}

#[repr(C)]
enum ApicRegisters
{
	ApicReg_LAPIC_ID  = 0x2,
	ApicReg_LAPIC_Ver = 0x3,
	ApicReg_TPR       = 0x8,
	ApicReg_APR       = 0x9,
	ApicReg_PPR       = 0xA,
}

fn init()
{
}

impl APIC
{
	pub fn init() -> Option<APIC>
	{
		Some(APIC {
			regs: unsafe { &mut s_lapic_mapping }
			})
	}
	
}


// vim: ft=rust

