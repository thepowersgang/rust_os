// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/fadt.rs
//! Fixed ACPI Description Table

use super::super::Fadt;

struct _FadtExtra
{
	orig: Fadt,
 
	reset_reg: super::GAS,
	reset_val: u8,
	_rsvd3: [u8; 3],
	
	// 64bit pointers - Available on ACPI 2.0+
	x_firmware_control: u64,
	x_dsdt: u64,
	
	x_pm1a_event_block: super::GAS,
	x_pm1b_event_block: super::GAS,
	x_pm1a_control_block: super::GAS,
	x_pm1b_control_block: super::GAS,
	x_pm2_control_block: super::GAS,
	x_pm_timer_block: super::GAS,
	x_gpe0_block: super::GAS,
	x_gpe1_block: super::GAS,
}


pub fn parse_fadt()
{
	let fadt = super::find_table::<Fadt>("FACP", 0).unwrap();
	
	log_debug!("DSDT: {:#x}", {fadt.data().dsdt_addr});
	
	let dsdt_paddr = fadt.data().dsdt_addr as crate::memory::PAddr;
	
	// SAFE: Trusting the DSDT address to be correct
	let dsdt = unsafe { super::SDTHandle::<()>::new( dsdt_paddr ) };
	crate::logging::hex_dump_t( "DSDT ", &*dsdt );
	if &dsdt.raw_signature()[..] != b"DSDT" || !dsdt.validate() {
		log_warning!("DSDT is invalid");
	}
	let dsdt_bytes = dsdt.data_byte_slice();

	
	if false {
		super::aml::dump_aml(dsdt_bytes);
	}
}

