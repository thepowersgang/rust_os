// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/fadt.rs
//! Fixed ACPI Description Table
use _common::*;

#[repr(C,packed)]
#[allow(dead_code)]
struct Fadt
{
	/// Pointer to the FACS
	firmware_ctrl:	u32,
	/// Pointer to the DSTD
	dsdt_addr: u32,
	
	_rsvd1:	u8,
	
	preferred_power_mgmt_profile: u8,
	sci_interrupt:    u16,
	smi_command_port: u32,
	acpi_enable:	u8,
	acpi_disable:	u8,
	s4bios_req:	u8,
	pstate_control:	u8,
	
	pm1a_event_block: u32,
	pm1b_event_block: u32,
	pm1a_control_block: u32,
	pm1b_control_block: u32,
	pm2_control_block: u32,
	pm_timer_block: u32,
	gpe0_block: u32,
	gpe1_block: u32,
	
	pm1_event_length: u8,
	pm1_control_length: u8,
	pm2_control_length: u8,
	pm_timer_length: u8,
	gpe0_length: u8,
	gpe1_length: u8,
	gpe1_base: u8,

	cstate_control: u8,
	worst_c2_latency: u16,
	worst_c3_latency: u16,
	flush_size: u16,
	flush_stride: u16,
	duty_offset:	u8,
	duty_width:	u8,
	day_alarm:	u8,
	month_alarm:	u8,
	century:	u8,
	
	// reserved in ACPI 1.0; used since ACPI 2.0+
	boot_architecture_flags: u16,
	_rsvd2: u8,
	
	flags: u32,
}

struct FadtExtra
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
	
	log_debug!("DSDT: {:#x}", fadt.data().dsdt_addr);
	
	let dsdt_paddr = fadt.data().dsdt_addr as ::memory::PAddr;
	
	let dsdt = super::SDTHandle::<()>::new( dsdt_paddr );
	::logging::hex_dump_t( "DSDT ", &*dsdt );
	if &dsdt.raw_signature()[..] != b"DSDT" || !dsdt.validate() {
		log_warning!("DSDT is invalid");
	}
	let dsdt_bytes = unsafe { dsdt.data_byte_slice() };

	
	if false {
		super::aml::dump_aml(dsdt_bytes);
	}
}

