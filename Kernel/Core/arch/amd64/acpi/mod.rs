// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/mod.rs
//! ACPI (Advanced Control and Power Interface) handling code
//!
//! Provides access to the ACPI tables
#[allow(unused_imports)]
use crate::prelude::*;

module_define!{ACPI, [], init}

#[cfg_attr(any(use_acpica,feature="acpica"), path="acpica/mod.rs")]
#[cfg_attr(not(any(use_acpica,feature="acpica")), path="mine/mod.rs")]
mod internal;

#[repr(u8)]
#[derive(Copy,Clone,PartialEq)]
/// Address space identifier
pub enum AddressSpaceID
{
	/// Memory-mapped IO
	Memory   = 0,
	/// x86 IO bus
	IO       = 1,
	/// PCI configuration space
	PCI      = 2,
	Embedded = 3,
	SMBus    = 4,
	PCC      = 0xA,
	FFH      = 0x7F,
}

#[repr(C,packed)]
#[derive(Copy,Clone)]
/// Generic address descriptor (TODO: check name)
pub struct GAS
{
	pub asid: u8,	///! Address space ID
	pub bit_width: u8,
	pub bit_ofs: u8,
	pub access_size: u8,	// 0: undef, 1: byte; ., 4: qword
	pub address: u64,
}

#[repr(C)]
#[derive(Copy,Clone)]
pub struct SDTHeader
{
	pub signature: [u8; 4],
	pub length: u32,
	pub revision: u8,
	pub checksum: u8,
	pub oemid: [u8; 6],
	pub oem_table_id: [u8; 8],
	pub oem_revision: u32,
	pub creator_id: u32,
	pub creator_revision: u32,
}

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

#[repr(C)]
/// A generic descriptor table
pub struct SDT<T:'static>
{
	header: SDTHeader,
	data: T
}
unsafe impl<T: crate::lib::POD> crate::lib::POD for SDT<T> {}

fn init()
{
	internal::init();
}


use self::internal::SDTHandle;

pub fn find<T: crate::lib::POD>(name: &str, idx: usize) -> Option<SDTHandle<T>> {
	internal::find_table(name, idx)
}
pub fn count(name: &str) -> usize {
	internal::count_tables(name)
}

impl<T> SDT<T>
{
	#[allow(dead_code)]
	fn validate(&self) -> bool
	{
		if ::core::mem::size_of::<Self>() != self.header.length as usize {
			log_notice!("SDT size mismatch {} != sizeof({}) {}",
				self.header.length, type_name!(SDT<T>), ::core::mem::size_of::<Self>());
		}
		if ! crate::memory::buf_valid(self as *const _ as *const (), self.header.length as usize) {
			log_warning!("SDT<{}> ({} bytes reported) not all in valid memory", type_name!(T), self.header.length);
			false
		}
		else {
			// SAFE: Self is POD
			unsafe {
				let bytes = ::core::slice::from_raw_parts(self as *const _ as *const u8, self.header.length as usize);
				bytes.iter().fold(0u8, |a,&b| a.wrapping_add(b)) == 0
			}
		}
	}
	#[allow(dead_code)]
	fn raw_signature(&self) -> [u8; 4]
	{
		crate::arch::amd64::checkmark();
		self.header.signature
	}
	pub fn data_len(&self) -> usize
	{
		self.header.length as usize - ::core::mem::size_of::<SDTHeader>()
	}
	pub fn header(&self) -> &SDTHeader {
		&self.header
	}
	pub fn data<'s>(&'s self) -> &'s T {
		&self.data
	}
	
	pub fn data_byte_slice(&self) -> &[u8] {
		// SAFE: T should be POD
		unsafe {
			::core::slice::from_raw_parts(&self.data as *const _ as *const u8, self.data_len())
		}
	}
}


