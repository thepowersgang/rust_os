// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/mod.rs
//! ACPI (Advanced Control and Power Interface) handling code
//!
//! Provides access to the ACPI tables
#[allow(unused_imports)]
use crate::prelude::*;
use crate::lib::byte_str::ByteStr;

module_define!{ACPI, [], init}
mod late {
	module_define!{ACPI_Late, [APIC], super::init_late}
}

#[cfg_attr(any(use_acpica,feature="acpica"), path="acpica/mod.rs")]
#[cfg_attr(not(any(use_acpica,feature="acpica")), path="mine/mod.rs")]
mod internal;

pub mod tables;

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
impl ::core::fmt::Debug for SDTHeader
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		write!(f, "SDTHeader = {{ sig:{:?},length='{}',rev={},checksum={},  oemid={:?},oem_table_id={:?},oem_revision={}, creator_id={:#x}, creator_revision={} }}",
			ByteStr::new(&self.signature), self.length, self.revision, self.checksum,
			ByteStr::new(&self.oemid), ByteStr::new(&self.oem_table_id), self.oem_revision,
			self.creator_id, self.creator_revision)
	}
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
fn init_late() {
	internal::init_late();
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
	pub fn data_len(&self) -> usize {
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

	pub fn iterate(&self) -> T::Iter<'_> where T: tables::Table {
		self.data.iterate_subitems(&self.data_byte_slice()[::core::mem::size_of::<Self>()..])
	}
}
