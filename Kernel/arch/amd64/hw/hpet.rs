// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/hw/hpet.rs
// - x86 High Precision Event Timer
use _common::*;

module_define!(HPET, [ACPI], init)

#[repr(C)]
struct ACPI_HPET
{
	hw_rev_id: u8,
	flags: u8,
	pci_vendor: u16,
	
	// address_structure
	asid: u8,
	bit_width: u8,
	bit_offset: u8,
	_rsvd: u8,
	address: u64,
	// /address_structure
	hpet_num: u8,
	mintick: [u8,..2],
	page_protection: u8,
}

fn init()
{
	log_trace!("init()");
	let handles = ::arch::acpi::find::<ACPI_HPET>("HPET");
	
}

// vim: ft=rust

