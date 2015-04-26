// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/mod.rs
//! ACPI (Advanced Control and Power Interface) handling code
//!
//! Provides access to the ACPI tables
use _common::*;

module_define!{ACPI, [], init}

#[cfg(use_acpica)]
#[path="mod_acpica.rs"] mod internal;
#[cfg(not(use_acpica))]
#[path="mod_mine.rs"] mod internal;

#[repr(C,u8)]
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
struct SDTHeader
{
	signature: [u8; 4],
	length: u32,
	revision: u8,
	checksum: u8,
	oemid: [u8; 6],
	oem_table_id: [u8; 8],
	oem_revision: u32,
	creator_id: u32,
	creator_revision: u32,
}


#[repr(C)]
/// A generic descriptor table
pub struct SDT<T:'static>
{
	header: SDTHeader,
	data: T
}

fn init()
{
	internal::init();
}


use self::internal::SDTHandle;

pub fn find<T>(name: &str, idx: usize) -> Option<SDTHandle<T>>
{
	internal::find_table(name, idx)
}
pub fn count(name: &str) -> usize {
	internal::count_tables(name)
}

