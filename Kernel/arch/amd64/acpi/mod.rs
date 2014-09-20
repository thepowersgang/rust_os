//
//
//
use _common::*;
use core::ptr::RawPtr;

module_define!(ACPI, [], init)

struct ACPI
{
	top_sdt: TLSDT
	//names: Vec<[u8,..4]>,
}

enum TLSDT
{
	TopRSDT(&'static SDT<RSDT>),
	TopXSDT(&'static SDT<XSDT>),
}

#[repr(packed)]
struct RSDP
{
	signature: [u8,..8],
	checksum: u8,
	oemid: [u8,..6],
	revision: u8,
	rsdt_address: u32,
	// Version 2.0
	length: u32,
	xsdt_address: u64,
	ext_checksum: u8,
	_resvd1: [u8,..3],
}

/// A handle to a SDT
pub struct SDTHandle<T:'static>
{
	maphandle: ::memory::virt::AllocHandle,
	ofs: uint,
}

#[repr(C)]
pub struct SDTHeader
{
	signature: [u8, ..4],
	length: u32,
	revision: u8,
	checksum: u8,
	oemid: [u8, ..6],
	oem_table_id: [u8, ..8],
	oem_revision: u32,
	creator_id: u32,
	creator_revision: u32,
}

#[repr(C)]
pub struct SDT<T:'static>
{
	header: SDTHeader,
	data: T
}

#[repr(C)]
struct RSDT
{
	pointers: u32,
}

#[repr(C)]
struct XSDT
{
	pointers: u64,	// Rust doesn't support arbitary length arrays
}

static mut s_acpi_state : Option<ACPI> = None;

/// ACPI module init - Locate the [RX]SDT
fn init()
{
	let rsdp = match get_rsdp() {
		Some(x) => x,
		None => {
			log_notice!("Cannot find RSDP");
			return;
			}
		};
	log_debug!("RSDP.oemid = {}", ::core::str::from_utf8(rsdp.oemid));
	log_debug!("RSDP.revision = {:#x}", rsdp.revision);
	log_debug!("RSDP.rsdt_address = {:#x}", rsdp.rsdt_address);
	
	let tl = if rsdp.revision == 0 {
			TopRSDT( SDTHandle::<RSDT>::new( rsdp.rsdt_address as u64 ).make_static() )
		} else {
			TopXSDT( SDTHandle::<XSDT>::new( rsdp.xsdt_address ).make_static() )
		};
	log_debug!("*SDT.signature = {}", tl.signature());
	log_debug!("*SDT.oemid = '{}'", tl.oemid());
	
//	let names = range(0,tl.len()).map(|i| tl.get::<SDTHeader>(i).name()).collect();
	
	unsafe {
		s_acpi_state = Some( ACPI {
			top_sdt: tl,
			//names: names,
			});
	}
}

pub fn find<T:'static>(name: &'static str) -> Vec<SDTHandle<T>>
{
	assert_eq!(name.len(), 4);
	let acpi = unsafe { &s_acpi_state.unwrap() };
	let mut ret = Vec::new();
	for i in range(0, acpi.top_sdt.len())
	{
		let r = acpi.top_sdt.get::<T>(i);
		log_debug!("r.header.name = {}", (*r).signature());
		if (*r).signature() == name
		{
			ret.push(r);
		}
	}
	ret
}

fn get_rsdp() -> Option<&'static RSDP>
{
	unsafe {
	let ebda_ver = locate_rsdp((::arch::memory::addresses::ident_start + 0x9FC00) as *const u8, 0x400);
	if !ebda_ver.is_null() {
		return ebda_ver.as_ref();
	}
	let bios_ver = locate_rsdp((::arch::memory::addresses::ident_start + 0xE0000) as *const u8, 0x20000);
	if !bios_ver.is_null() {
		return bios_ver.as_ref();
	}
	}
	return None;
}

unsafe fn locate_rsdp(base: *const u8, size: uint) -> *const RSDP
{
	for ofs in range_step(0, size, 16)
	{
		let sig = base.offset(ofs as int) as *const [u8,..8];
		if *sig == "RSD PTR ".as_bytes()
		{
			return sig as *const _;
		}
	}
	RawPtr::null()
}

impl TLSDT
{
	fn _header<'self_>(&'self_ self) -> &'self_ SDTHeader {
		match self {
		&TopRSDT(sdt) => &(*sdt).header,
		&TopXSDT(sdt) => &(*sdt).header,
		}
	}
	fn _getaddr(&self, idx: uint) -> u64 {
		unsafe {
		match self {
		&TopRSDT(sdt) => *((&(*sdt).data.pointers) as *const u32).offset(idx as int) as u64,
		&TopXSDT(sdt) => *(&(*sdt).data.pointers as *const u64).offset(idx as int),
		}
		}
	}
	
	fn len(&self) -> uint {
		(self._header().length as uint - ::core::mem::size_of::<SDTHeader>()) / match self {
			&TopRSDT(_) => 4,
			&TopXSDT(_) => 8,
			}
	}
	
	fn signature<'self_>(&'self_ self) -> &'self_ str {
		::core::str::from_utf8(self._header().signature).unwrap()
	}
	fn oemid<'self_>(&'self_ self) -> &'self_ str {
		::core::str::from_utf8(self._header().oemid).unwrap()
	}
	fn get<T>(&self, idx: uint) -> SDTHandle<T> {
		SDTHandle::<T>::new(self._getaddr(idx))
	}
}

impl<T> SDTHandle<T>
{
	/// Map an SDT into memory, given a physical address
	pub fn new(physaddr: u64) -> SDTHandle<T>
	{
		let ofs = (physaddr & (::PAGE_SIZE - 1) as u64) as uint;
		
		// Obtain length (and validate)
		let (length,) = SDTHandle::<T>::_get_info(physaddr, ofs);
		
		// Map the resultant memory
		let npages = (ofs + length + ::PAGE_SIZE - 1) / ::PAGE_SIZE;
		let maphandle = match ::memory::virt::map_hw_ro(physaddr - ofs as u64, npages, "ACPI") {
			Ok(x) => x,
			Err(_) => fail!("Map fail")
			};
		SDTHandle {
			maphandle: maphandle,
			ofs: ofs
			}
	}
	
	fn _get_info(physaddr: u64, ofs: uint) -> (uint,)
	{
		// TODO: Support the SDT header spanning acrosss two pages
		assert!(::PAGE_SIZE - ofs >= ::core::mem::size_of::<SDTHeader>());
		// Map the header into memory temporarily
		let tmp = match ::memory::virt::map_hw_ro(physaddr - ofs as u64, 1, "ACPI") {
			Ok(v) => v,
			Err(_) => fail!("Oops, temp mapping SDT failed"),
			};
		let hdr = tmp.as_ref::<SDTHeader>(ofs);
		
		// Validate and get the length
		// TODO: Can this code get the type name as a string?
		log_debug!("hdr.signature = {}", ::core::str::from_utf8(hdr.signature));
		log_debug!("hdr.length = {:#x}", hdr.length);
		
		(hdr.length as uint,)
	}
	
	pub fn make_static(&mut self) -> &'static SDT<T>
	{
		self.maphandle.make_static::<SDT<T>>(self.ofs)
	}
}

impl<T> Deref<SDT<T>> for SDTHandle<T>
{
	fn deref<'s>(&'s self) -> &'s SDT<T> {
		self.maphandle.as_ref(self.ofs)
	}
}

impl<T> SDT<T>
{
	fn signature<'s>(&'s self) -> &'s str
	{
		::core::str::from_utf8(self.header.signature).unwrap()
	}
}

// vim: ft=rust

