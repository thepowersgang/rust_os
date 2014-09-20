// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/hw/hpet.rs
// - x86 High Precision Event Timer
use _common::*;

module_define!(HPET, [ACPI], init)

struct HPET
{
	mapping_handle: ::memory::virt::AllocHandle,
}

#[repr(C,packed)]
struct ACPI_HPET
{
	hw_rev_id: u8,
	flags: u8,
	pci_vendor: u16,
	addr: ::arch::acpi::GAS,
	hpet_num: u8,
	mintick: [u8,..2],	// 16-bit word
	page_protection: u8,
}

#[repr(C,packed)]
struct HPETRegs
{
	caps_id: u64, _r1: u64,
	config: u64,  _r2: u64,
	isr: u64, _r3: u64,
	_rsvd3: [u64, ..24],
	main_counter: u64, _r4: u64,
	comparitors: [HPETComparitorRegs,..32],
}

#[repr(C,packed)]
struct HPETComparitorRegs
{
	config_caps: u64,
	value: u64,
	int_route: u64,
	_rsvd: u64,
}

static mut s_instance : *mut HPET = 0 as *mut _;

fn init()
{
	log_trace!("init()");
	let handles = ::arch::acpi::find::<ACPI_HPET>("HPET");
	if handles.len() == 0 {
		log_error!("No HPET, in ACPI, no timing avaliable");
		return ;
	}
	if handles.len() != 1 {
		log_warning!("Multiple HPETs not yet supported, using first only");
	}
	let hpet = &handles[0];

	let info = (*hpet).data();
	log_debug!("-- HPET {} --", info.hpet_num);
	log_debug!("Rev/Flags/Vendor = {}/{:#x}/{:x}", info.hw_rev_id, info.flags, info.pci_vendor);
	log_debug!("asid:address = {}:{:#x}", info.addr.asid, info.addr.address);
	
	assert!(info.addr.asid == 0);
	assert!(info.addr.address % ::PAGE_SIZE as u64 == 0);
	let mapping = ::memory::virt::map_hw_rw(info.addr.address, 1, "HPET").unwrap();
	{
		let regs = mapping.as_ref::<HPETRegs>(0);
		log_debug!("Capabilities = {:#016x}", regs.caps_id);
		log_debug!(" > Period = {}fS, Vendor = {:04x}, Legacy? = {}, Count = {}, 64-bit? = {}, Rev = {}",
				regs.caps_id >> 32, (regs.caps_id >> 16) & 0xFFFF,
				(regs.caps_id >> 15) & 1, (regs.caps_id >> 13) & 1, (regs.caps_id >> 8) & 0x1F, regs.caps_id & 0xFF
				);
		log_debug!("Config = {:#x}", regs.config);
		log_debug!("ISR = {:#x}", regs.isr);
		log_debug!("Main Counter = {:#x}", regs.main_counter);
		
		log_debug!("Cmp0 = {{ {:#x} {:#x} {:#x}  }}",
			regs.comparitors[0].config_caps, regs.comparitors[0].value, regs.comparitors[0].int_route);
		log_debug!("Cmp1 = {{ {:#x} {:#x} {:#x}  }}",
			regs.comparitors[1].config_caps, regs.comparitors[1].value, regs.comparitors[1].int_route);
	}
	
	unsafe {
		s_instance = ::memory::heap::alloc( HPET {
			mapping_handle: mapping
			} );
	}
}

// vim: ft=rust

