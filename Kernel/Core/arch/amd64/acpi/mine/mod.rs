// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/mod_mine.rs
//! ACPI Component Architecture binding
use crate::prelude::*;
use crate::lib::byte_str::ByteStr;

use self::TLSDT::{TopRSDT,TopXSDT};
use super::{SDT,SDTHeader};
pub use super::GAS;

mod fadt;
mod aml;
mod sdt_handle;

pub use self::sdt_handle::SDTHandle;

struct ACPI
{
	top_sdt: TLSDT,
	names: Vec<[u8; 4]>,
}

enum TLSDT
{
	TopRSDT(&'static SDT<RSDT>),
	TopXSDT(&'static SDT<XSDT>),
}

#[repr(C,packed)]
struct RSDP
{
	signature: [u8; 8],
	checksum: u8,
	oemid: [u8; 6],
	revision: u8,
	rsdt_address: u32,
}
#[repr(C,packed)]
struct RSDPv2
{
	v1: RSDP,
	// Version 2.0
	length: u32,
	xsdt_address: u64,
	ext_checksum: u8,
	_resvd1: [u8; 3],
}

#[repr(C)]
struct RSDT
{
	pointers: u32,
}

#[repr(C)]
struct XSDT
{
	pointers: u64,	// Rust doesn't support arbitrary length arrays
}

static S_ACPI_STATE: crate::lib::LazyStatic<ACPI> = lazystatic_init!();

/// ACPI module init - Locate the [RX]SDT
pub fn init()
{
	let rsdp = match get_rsdp() {
		Some(x) => x,
		None => {
			log_notice!("Cannot find RSDP");
			return;
			}
		};
	log_debug!("RSDP = {:p} {{ oemid = {:?}, revision = {:#x}, rsdt_address = {:#x} }}",
		rsdp,
		::core::str::from_utf8(&rsdp.oemid), rsdp.revision, {rsdp.rsdt_address});
	
	// Determine the top-level SDT type
	// SAFE: The addresses are valid (well, they should be, the checksums passed)
	let tl = unsafe {
			if rsdp.revision == 0 {
				TopRSDT( SDTHandle::<RSDT>::new( rsdp.rsdt_address as u64 ).make_static() )
			} else {
				let v2: &RSDPv2 = &*(rsdp as *const _ as *const _);
				if sum_struct(v2) != 0 {
					// oh
					panic!("RSDPv2 checksum failed");
				}
				TopXSDT( SDTHandle::<XSDT>::new( v2.xsdt_address ).make_static() )
			}
		};
	log_debug!("*SDT = {{ signature = {:?}, oemid = {:?} }}", tl.signature(), tl.oemid());
	if !tl.validate() {
		log_error!("Root SDT failed validation");
		return ;
	}
	
	// Obtain list of SDTs (signatures only)
	let names = (0 .. tl.len()).map( |i| tl.get::<SDTHeader>(i).raw_signature() ).collect();
	
	S_ACPI_STATE.prep(|| ACPI { top_sdt: tl, names, });
	
	
	// Poke sub-enumerators
	fadt::parse_fadt();
}
pub fn init_late() {
}

/// Find all SDTs with a given signature
pub fn find_table<T: 'static + crate::lib::POD>(req_name: &str, mut idx: usize) -> Option<SDTHandle<T>>
{
	log_debug!("find('{}',{})", req_name, idx);
	assert_eq!(req_name.len(), 4);
	for (i,ent_name) in S_ACPI_STATE.names.iter().enumerate()
	{
		if &ent_name[..] != req_name.as_bytes()
		{
		}
		else if idx > 0
		{
			idx -= 1;
		}
		else
		{
			let table = S_ACPI_STATE.top_sdt.get::<T>(i);
			if (*table).validate() == false {
				log_error!("ACPI ent #{} failed checksum", i);
			}
			return Some( table );
		}
	}
	None
}
pub fn count_tables(req_name: &str) -> usize {
	assert_eq!(req_name.len(), 4);
	S_ACPI_STATE.names.iter().filter(|n| &n[..] == req_name.as_bytes()).count()
}

/// Obtain a reference to the RSDP (will be in the identity mapping area)
fn get_rsdp() -> Option<&'static RSDP>
{
	// SAFE: Valid pointers are passed
	unsafe {
		let ebda_ver = locate_rsdp((crate::arch::amd64::memory::addresses::IDENT_START + 0x9_FC00) as *const u8, 0x400);
		if !ebda_ver.is_null() {
			return ebda_ver.as_ref();
		}
		let bios_ver = locate_rsdp((crate::arch::amd64::memory::addresses::IDENT_START + 0xE_0000) as *const u8, 0x2_0000);
		if !bios_ver.is_null() {
			return bios_ver.as_ref();
		}
	}
	return None;
}
/// Search a section of memory for the RSDP
unsafe fn locate_rsdp(base: *const u8, size: usize) -> *const RSDP
{
	//for ofs in (0 .. size).step_by(16)
	for i in 0 .. (size/16)
	{
		let ofs = i * 16;
		let sig = base.offset(ofs as isize) as *const [u8; 8];
		if &*sig == b"RSD PTR "
		{
			let ret = sig as *const RSDP;
			if sum_struct(&*ret) == 0
			{
				return ret;
			}
		}
	}
	::core::ptr::null()
}

/// Calculate the byte sum of a structure
fn sum_struct<T: crate::lib::POD>(s: &T) -> u8
{
	// SAFE: T is POD
	unsafe {
		let ptr = s as *const T as *const u8;
		let vals = ::core::slice::from_raw_parts(ptr, ::core::mem::size_of::<T>());
		vals.iter().fold(0, |a,&b| a+b)
	}
}

impl TLSDT
{
	fn header(&self) -> &SDTHeader {
		match self {
		&TopRSDT(sdt) => &(*sdt).header,
		&TopXSDT(sdt) => &(*sdt).header,
		}
	}
	unsafe fn getaddr(&self, idx: usize) -> u64 {
		match self {
		&TopRSDT(sdt) => (*sdt).getptr(idx),
		&TopXSDT(sdt) => (*sdt).getptr(idx),
		}
	}
	fn validate(&self) -> bool {
		match self {
		&TopRSDT(sdt) => sdt.validate(),
		&TopXSDT(sdt) => sdt.validate(),
		}
	}
	
	fn len(&self) -> usize {
		(self.header().length as usize - ::core::mem::size_of::<SDTHeader>()) / match self {
			&TopRSDT(_) => 4,
			&TopXSDT(_) => 8,
			}
	}
	
	fn signature(&self) -> &ByteStr {
		ByteStr::new(&self.header().signature)
	}
	fn oemid(&self) -> &ByteStr {
		ByteStr::new(&self.header().oemid)
	}
	fn get<T: crate::lib::POD>(&self, idx: usize) -> SDTHandle<T> {
		// SAFE: Immutable access, and address is as validated as can be
		unsafe {
			assert!(idx < self.len());
			SDTHandle::<T>::new(self.getaddr(idx))
		}
	}
}
trait RSDTTrait
{
	unsafe fn getptr(&self, idx: usize) -> u64;
}

impl RSDTTrait for SDT<RSDT>
{
	unsafe fn getptr(&self, idx: usize) -> u64
	{
		let ptrs = &(self.data.pointers) as *const u32;
		assert!( !ptrs.is_null() );
		*ptrs.offset(idx as isize) as u64
	}
}
impl RSDTTrait for SDT<XSDT>
{
	unsafe fn getptr(&self, idx: usize) -> u64
	{
		let ptrs = &(self.data.pointers) as *const u64;
		assert!( !ptrs.is_null() );
		*ptrs.offset(idx as isize)
	}
}


// vim: ft=rust
