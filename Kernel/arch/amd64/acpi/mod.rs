// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/acpi/mod.rs
// - ACPI (Advanced Control and Power Interface) handling code
// 
// > Provides access to the ACPI tables
use _common::*;
use core::ptr::RawPtr;
use core::str::from_utf8;

module_define!(ACPI, [], init)

struct ACPI
{
	top_sdt: TLSDT,
	names: Vec<[u8,..4]>,
}

enum TLSDT
{
	TopRSDT(&'static SDT<RSDT>),
	TopXSDT(&'static SDT<XSDT>),
}

#[repr(C,packed)]
struct RSDP
{
	signature: [u8,..8],
	checksum: u8,
	oemid: [u8,..6],
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
pub enum AddressSpaceID
{
	AsidMemory   = 0,
	AsidIO       = 1,
	AsidPCI      = 2,
	AsidEmbedded = 3,
	AsidSMBus    = 4,
	AsidPCC      = 0xA,
	AsidFFH      = 0x7F,
}

#[repr(C,packed)]
pub struct GAS
{
	pub asid: u8,
	pub bit_width: u8,
	pub bit_ofs: u8,
	pub access_size: u8,	// 0: undef, 1: byte, ..., 4: qword
	pub address: u64,
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

static mut s_acpi_state : *const ACPI = 0 as *const _;

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
	log_debug!("RSDP = {{ oemid = {}, revision = {:#x}, rsdt_address = {:#x} }}",
		::core::str::from_utf8(rsdp.oemid), rsdp.revision, rsdp.rsdt_address);
	
	CHECKMARK!();
	let tl = if rsdp.revision == 0 {
			TopRSDT( SDTHandle::<RSDT>::new( rsdp.rsdt_address as u64 ).make_static() )
		} else {
			let v2: &RSDPv2 = unsafe { ::core::mem::transmute(rsdp) };
			if sum_struct(v2) != 0 {
				// oh
			}
			TopXSDT( SDTHandle::<XSDT>::new( v2.xsdt_address ).make_static() )
		};
	log_debug!("*SDT = {{ signature = {}, oemid = '{}' }}", tl.signature(), tl.oemid());
	
	CHECKMARK!();
	// Obtain list of SDTs (signatures only)
	let names = range(0, tl.len()).map(
		|i| {
			tl.get::<SDTHeader>(i).raw_signature()
			}
		).collect();
	
	CHECKMARK!();
	unsafe {
		s_acpi_state = ::memory::heap::alloc( ACPI {
			top_sdt: tl,
			names: names,
			}) as *const ACPI;
	}
}

/// Find all SDTs with a given signature
pub fn find<T:'static>(req_name: &'static str) -> Vec<SDTHandle<T>>
{
	assert_eq!(req_name.len(), 4);
	let acpi = unsafe { s_acpi_state.as_ref().unwrap() };
	let mut ret = Vec::new();
	for (i,ent_name) in acpi.names.iter().enumerate()
	{
		log_debug!("ent {} name = {}", i, from_utf8(ent_name));
		if from_utf8(ent_name).unwrap() != req_name {
			continue ;
		}
		
		let table = acpi.top_sdt.get::<T>(i);
		if (*table).validate() == false {
			log_error!("ACPI ent #{} failed checksum", i);
		}
		ret.push(table);
	}
	ret
}

/// Obtain a reference to the RSDP (will be in the identity mapping area)
fn get_rsdp() -> Option<&'static RSDP>
{
	unsafe {
	let ebda_ver = locate_rsdp((::arch::memory::addresses::IDENT_START + 0x9FC00) as *const u8, 0x400);
	if !ebda_ver.is_null() {
		return ebda_ver.as_ref();
	}
	let bios_ver = locate_rsdp((::arch::memory::addresses::IDENT_START + 0xE0000) as *const u8, 0x20000);
	if !bios_ver.is_null() {
		return bios_ver.as_ref();
	}
	}
	return None;
}
/// Search a section of memory for the RSDP
unsafe fn locate_rsdp(base: *const u8, size: uint) -> *const RSDP
{
	for ofs in range_step(0, size, 16)
	{
		let sig = base.offset(ofs as int) as *const [u8,..8];
		if *sig == "RSD PTR ".as_bytes()
		{
			let ret = sig as *const RSDP;
			if sum_struct(&*ret) == 0
			{
				return ret;
			}
		}
	}
	RawPtr::null()
}

/// Caclulate the byte sum of a structure
fn sum_struct<T>(s: &T) -> u8
{
	let ptr = s as *const T as *const u8;
	unsafe { ::core::slice::raw::buf_as_slice(
		ptr,
		::core::mem::size_of::<T>(),
		|vals| vals.iter().fold(0, |a,&b| a+b)
		)}
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
		match self {
		&TopRSDT(sdt) => (*sdt).getptr(idx),
		&TopXSDT(sdt) => (*sdt).getptr(idx),
		}
	}
	
	fn len(&self) -> uint {
		(self._header().length as uint - ::core::mem::size_of::<SDTHeader>()) / match self {
			&TopRSDT(_) => 4,
			&TopXSDT(_) => 8,
			}
	}
	
	fn signature<'self_>(&'self_ self) -> &'self_ str {
		from_utf8(self._header().signature).unwrap()
	}
	fn oemid<'self_>(&'self_ self) -> &'self_ str {
		from_utf8(self._header().oemid).unwrap()
	}
	fn get<T>(&self, idx: uint) -> SDTHandle<T> {
		SDTHandle::<T>::new(self._getaddr(idx))
	}
}
trait RSDTTrait
{
	fn getptr(&self, idx: uint) -> u64;
}

impl RSDTTrait for SDT<RSDT>
{
	fn getptr(&self, idx: uint) -> u64
	{
		let ptrs = &(self.data.pointers) as *const u32;
		assert!( !ptrs.is_null() );
		unsafe {
			*ptrs.offset(idx as int) as u64
		}
	}
}
impl RSDTTrait for SDT<XSDT>
{
	fn getptr(&self, idx: uint) -> u64
	{
		let ptrs = &(self.data.pointers) as *const u64;
		assert!( !ptrs.is_null() );
		unsafe {
			*ptrs.offset(idx as int)
		}
	}
}

impl SDTHeader
{
	pub fn validate_checksum(&self) -> bool
	{
		// TODO: This checksum is over the entire table!
		let sum = sum_struct(self);
		(sum & 0xFF) == 0
	}
	pub fn dump(&self)
	{
		log_debug!("SDTHeader = {{ sig:{},length='{}',rev={},checksum={},...",
			from_utf8(self.signature), self.length, self.revision, self.checksum);
		log_debug!(" oemid={},oem_table_id={},oem_revision={},...",
			from_utf8(self.oemid), from_utf8(self.oem_table_id), self.oem_revision);
		log_debug!(" creator_id={:#x}, creator_revision={}",
			self.creator_id, self.creator_revision);
	}
}

impl<T> SDTHandle<T>
{
	/// Map an SDT into memory, given a physical address
	pub fn new(physaddr: u64) -> SDTHandle<T>
	{
		log_trace!("new(physaddr={:#x})", physaddr);
		let ofs = (physaddr & (::PAGE_SIZE - 1) as u64) as uint;
		
		// Obtain length (and validate)
		// TODO: Support the SDT header spanning acrosss two pages
		assert!(::PAGE_SIZE - ofs >= ::core::mem::size_of::<SDTHeader>());
		// Map the header into memory temporarily (maybe)
		let mut handle = match ::memory::virt::map_hw_ro(physaddr - ofs as u64, 1, "ACPI") {
			Ok(v) => v,
			Err(_) => fail!("Oops, temp mapping SDT failed"),
			};
		let (length,) = {
			let hdr = handle.as_ref::<SDTHeader>(ofs);
			
			// Get the length
			(hdr.length as uint,)
			};
		
		// Map the resultant memory
		let npages = (ofs + length + ::PAGE_SIZE - 1) / ::PAGE_SIZE;
		if npages != 1
		{
			handle = match ::memory::virt::map_hw_ro(physaddr - ofs as u64, npages, "ACPI") {
				Ok(x) => x,
				Err(_) => fail!("Map fail")
				};
		}
		SDTHandle {
			maphandle: handle,
			ofs: ofs
			}
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
	fn validate(&self) -> bool
	{
		sum_struct(self) == 0
	}
	//fn signature<'s>(&'s self) -> &'s str
	//{
	//	from_utf8(self.header.signature).unwrap()
	//}
	fn raw_signature(&self) -> [u8,..4]
	{
		CHECKMARK!();
		self.header.signature
	}
	pub fn data_len(&self) -> uint
	{
		self.header.length as uint - ::core::mem::size_of::<SDTHeader>()
	}
	pub fn data<'s>(&'s self) -> &'s T
	{
		&self.data
	}
}

// vim: ft=rust

