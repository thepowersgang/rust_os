//
//
//
use _common::*;
use core::ptr::RawPtr;

module_define!(ACPI, [], init)

struct ACPI<'a>
{
	top_sdt: TLSDT<'a>,
}

enum TLSDT<'a>
{
	TopRSDT(&'a RSDT),
	TopXSDT(&'a XSDT),
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

#[repr(C)]
struct SDTHeader
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
struct RSDT
{
	header: SDTHeader,
	pointers: [u32, ..0],
}

#[repr(C)]
struct XSDT
{
	header: SDTHeader,
	pointers: [u64, ..0],	// Lies, but rust doesn't support arbitary length arrays
}

pub fn init()
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
			TopRSDT( unsafe { ::core::mem::transmute(0 as *const RSDT) } )
		} else {
			TopXSDT( unsafe { ::core::mem::transmute(0 as *const XSDT) } )
		};
	log_debug!("*SDT.signature = {}", tl.signature());
	log_debug!("*SDT.oemid = {}", tl.oemid());
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

impl<'a> TLSDT<'a>
{
	fn signature<'self_>(&'self_ self) -> &'self_ str
	{
		match self {
		&TopRSDT(sdt) => ::core::str::from_utf8(sdt.header.signature),
		&TopXSDT(sdt) => ::core::str::from_utf8(sdt.header.signature),
		}.unwrap()
	}
	fn oemid<'self_>(&'self_ self) -> &'self_ str
	{
		match self {
		&TopRSDT(sdt) => ::core::str::from_utf8((*sdt).header.signature),
		&TopXSDT(sdt) => ::core::str::from_utf8((*sdt).header.signature),
		}.unwrap()
	}
}

// vim: ft=rust

