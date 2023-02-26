
#[repr(C,packed)]
#[allow(dead_code)]
pub struct Fadt
{
	/// Pointer to the FACS
	pub firmware_ctrl: u32,
	/// Pointer to the DSTD
	pub dsdt_addr: u32,

	_rsvd1: u8,

	pub preferred_power_mgmt_profile: u8,
	pub sci_interrupt: u16,
	pub smi_command_port: u32,
	pub acpi_enable: u8,
	pub acpi_disable: u8,
	pub s4bios_req: u8,
	pub pstate_control: u8,

	pub pm1a_event_block: u32,
	pub pm1b_event_block: u32,
	pub pm1a_control_block: u32,
	pub pm1b_control_block: u32,
	pub pm2_control_block: u32,
	pub pm_timer_block: u32,
	pub gpe0_block: u32,
	pub gpe1_block: u32,

	pub pm1_event_length: u8,
	pub pm1_control_length: u8,
	pub pm2_control_length: u8,
	pub pm_timer_length: u8,
	pub gpe0_length: u8,
	pub gpe1_length: u8,
	pub gpe1_base: u8,

	pub cstate_control: u8,
	pub worst_c2_latency: u16,
	pub worst_c3_latency: u16,
	pub flush_size: u16,
	pub flush_stride: u16,
	pub duty_offset: u8,
	pub duty_width: u8,
	pub day_alarm: u8,
	pub month_alarm: u8,
	pub century: u8,

	// reserved in ACPI 1.0; used since ACPI 2.0+
	pub boot_architecture_flags: u16,
	_rsvd2: u8,

	pub flags: u32,
}
impl super::Table for Fadt {
	type Iter<'a> = ::core::iter::Empty::<()>;
	fn iterate_subitems<'s>(&'s self, _data: &'s [u8]) -> Self::Iter<'s> {
		::core::iter::empty()
	}
}

