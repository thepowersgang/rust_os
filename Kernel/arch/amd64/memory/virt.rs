//
//
//
use core::ptr::PtrExt;
use super::{PAddr};
use PAGE_SIZE;

static MASK_VBITS : uint = 0x0000FFFF_FFFFFFFF;

#[deriving(PartialEq,Show)]
enum PTEPos
{
	Absent,
	Page512G,
	Page1G,
	Page2M,
	Page4K,
}

struct PTE
{
	pos: PTEPos,
	data: *mut u64
}

unsafe fn get_entry(level: u8, index: uint, force_allocate: bool) -> PTE
{
	use arch::memory::addresses::FRACTAL_BASE;
	
	let pt_page = (FRACTAL_BASE & MASK_VBITS) / PAGE_SIZE;
	let tab_pt = FRACTAL_BASE as *mut u64;
	let tab_pd = tab_pt.offset( pt_page as int );
	let tab_pdp = tab_pd.offset( (pt_page >> (9)) as int );
	let tab_pml4 = tab_pdp.offset( (pt_page >> (9+9)) as int );
	//log_debug!("tab_pt = {}, tab_pd = {}, tab_pdp = {}, tab_pml4 = {}",
	//	tab_pt, tab_pd, tab_pdp, tab_pml4);
	
	//log_trace!("get_entry(level={}, index={:#x})", level, index);
	// NOTE: Does no checks on presence
	match level
	{
	0 => {
		assert!(index < 512*512*512*512);
		PTE::new(PTEPos::Page4K, tab_pt.offset(index as int))
		}
	1 => {
		assert!(index < 512*512*512);
		let rv = PTE::new(PTEPos::Page2M, tab_pd.offset(index as int));
		if !rv.is_present() && force_allocate {
			let ptr = tab_pt.offset(index as int * 512) as *mut ();
			log_debug!("Allocating for {} (PDE {})", ptr, index);
			::memory::phys::allocate( ptr );
		}
		rv
		},
	2 => {
		assert!(index < 512*512);
		let rv = PTE::new(PTEPos::Page1G, tab_pdp.offset(index as int));
		if !rv.is_present() && force_allocate {
			let ptr = tab_pd.offset(index as int * 512) as *mut ();
			log_debug!("Allocating for {} (PDPE {})", ptr, index);
			::memory::phys::allocate( ptr );
		}
		rv
		},
	3 => {
		assert!(index < 512);
		let rv = PTE::new(PTEPos::Page512G, tab_pml4.offset(index as int));
		if !rv.is_present() && force_allocate {
			::memory::phys::allocate( tab_pdp.offset(index as int * 512) as *mut () );
		}
		rv
		},
	_ => panic!("Passed invalid number to get_entry, {} > 3", level)
	}
}
unsafe fn get_page_ent(addr: uint, from_temp: bool, allocate: bool, large_ok: bool) -> PTE
{
	assert!( from_temp == false );
	let pagenum = (addr & MASK_VBITS) / PAGE_SIZE;
//	log_trace!("get_page_ent(addr={:#x}, from_temp={}, allocate={}), pagenum={:#x}", addr, from_temp, allocate, pagenum);

	let mut ent = get_entry(3, pagenum >> (9*3), allocate);
	// 1. Walk down page tables from PML4
	if !ent.is_present() {
		return PTE::null();
	}

	ent = get_entry(2, pagenum >> (9*2), allocate);
	if !ent.is_present() {
		return PTE::null();
	}
	if ent.is_large() {
		panic!("TODO: Support large pages (1GiB)");
	}

	ent = get_entry(1, pagenum >> (9*1), allocate);
	if !ent.is_present() {
		return PTE::null();
	}
	if ent.is_large() {
		log_debug!("Large page covering {:#x}", addr);
		if large_ok {
			return ent;
		}
		else {	
			PTE::null();
		}
	}

	return get_entry(0, pagenum, allocate)
}

pub fn is_reserved(addr: uint) -> bool
{
	unsafe {
		let pte = get_page_ent(addr, false, false, true);
		return !pte.is_null() && pte.is_reserved();
	}
}
pub fn map(addr: *mut (), phys: PAddr, prot: ::memory::virt::ProtectionMode)
{
	unsafe {
		let pte = get_page_ent(addr as uint, false, true, false);
		pte.set( phys, prot );
	}
}
pub fn unmap(addr: *mut ())
{
	unsafe {
		let pte = get_page_ent(addr as uint, false, false, false);
		pte.set( 0, ::memory::virt::ProtectionMode::Unmapped );
	}
}

static PF_PRESENT : u64 = 0x001;
static PF_LARGE   : u64 = 0x080;

impl PTE
{
	pub fn new(pos: PTEPos, ptr: *mut u64) -> PTE
	{
		PTE { pos: pos, data: ptr }
	}
	pub fn null() -> PTE {
		PTE { pos: PTEPos::Absent, data: PtrExt::null() }
	}

	pub fn is_null(&self) -> bool { self.pos == PTEPos::Absent }
	pub unsafe fn is_reserved(&self) -> bool { !self.is_null() && *self.data != 0 }
	pub unsafe fn is_present(&self) -> bool { !self.is_null() && *self.data & 1 != 0 }
	pub unsafe fn is_large(&self) -> bool { *self.data & (PF_PRESENT | PF_LARGE) == PF_LARGE|PF_PRESENT }
	
	//pub unsafe fn addr(&self) -> PAddr { *self.data & 0x7FFFFFFF_FFFFF000 }
	//pub unsafe fn set_addr(&self, paddr: PAddr) {
	//	assert!(!self.is_null());
	//	*self.data = (*self.data & !0x7FFFFFFF_FFFFF000) | paddr;
	//}
	
	pub unsafe fn set(&self, paddr: PAddr, prot: ::memory::virt::ProtectionMode) {
		assert!(!self.is_null());
		let flags: u64 = match prot
			{
			::memory::virt::ProtectionMode::Unmapped => 0,
			::memory::virt::ProtectionMode::KernelRO => (1<<63)|1,
			::memory::virt::ProtectionMode::KernelRW => (1<<63)|2|1,
			::memory::virt::ProtectionMode::KernelRX => 1,
			::memory::virt::ProtectionMode::UserRO => (1<<63)|4|1,
			::memory::virt::ProtectionMode::UserRW => (1<<63)|4|2|1,
			::memory::virt::ProtectionMode::UserRX => 4|1,
			};
		*self.data = (paddr & 0x7FFFFFFF_FFFFF000) | flags;
	}
}

impl ::core::fmt::Show for PTE
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		unsafe { write!(f, "PTE({}, *{}={:#x})", self.pos, self.data, *self.data) }
	}
}

// vim: ft=rust

