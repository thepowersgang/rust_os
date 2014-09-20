// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/hw/apic.rs
// - x86 APIC (Advanced Programmable Interrupt Controller) driver
use _common::*;

module_define!(APIC, [ACPI], init)

#[repr(C,packed)]
struct ACPI_MADT
{
	local_controller_addr: u32,
	flags: u32,
	end: (),
}

#[repr(C)]
struct APICReg
{
	data: u32,
	_rsvd: [u32,..3],
}

struct APIC
{
	mapping: ::memory::virt::AllocHandle,
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
	let handles = ::arch::acpi::find::<ACPI_MADT>("APIC");
	if handles.len() == 0 {
		log_warning!("No MADT ('APIC') table in ACPI");
		return ;
	}
	if handles.len() > 1 {
		log_notice!("Multiple MADTs ({})", handles.len());
	}
	
	let madt = &handles[0];
	madt.data().dump();
}

impl APIC
{
	pub fn init(paddr: u64) -> APIC
	{
		APIC {
			mapping: ::memory::virt::map_hw_rw(paddr, 1, "APIC").unwrap(),
			}
	}
	
}

impl ACPI_MADT
{
	fn dump(&self)
	{
		log_debug!("MADT = {{");
		log_debug!("  local_controller_addr: {:#x}", self.local_controller_addr);
		log_debug!("  flags: {:#x}", self.flags);
		log_debug!("}}");
	}
}


// vim: ft=rust

