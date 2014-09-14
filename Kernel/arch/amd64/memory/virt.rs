//
//
//

use core::ptr::RawPtr;
use super::{PAddr,VAddr};

static MASK_VBITS : uint = 0x0000FFFF_FFFFFFFF;

#[deriving(PartialEq,Show)]
enum PTEPos
{
	PTEPosAbsent,
	PTEPos512G,
	PTEPos1G,
	PTEPos2M,
	PTEPos4K,
}

struct PTE
{
	pos: PTEPos,
	data: *mut u64
}

unsafe fn get_entry(level: u8, index: uint) -> PTE
{
	use arch::memory::addresses::fractal_base;
	
	let pt_page = (fractal_base & MASK_VBITS) / ::PAGE_SIZE;
	let tab_pt = fractal_base as *mut u64;
	let tab_pd = tab_pt.offset( pt_page as int );
	let tab_pdp = tab_pd.offset( (pt_page >> (9)) as int );
	let tab_pml4 = tab_pdp.offset( (pt_page >> (9+9)) as int );
	//log_debug!("tab_pt = {}, tab_pd = {}, tab_pdp = {}, tab_pml4 = {}",
	//	tab_pt, tab_pd, tab_pdp, tab_pml4);
	
	//log_trace!("get_entry(level={}, index={:#x})", level, index);
	let fractal = fractal_base as *mut PTE;
	// NOTE: Does no checks on presence
	match level
	{
	0 => {
		assert!(index < 512*512*512*512)
		PTE::new(PTEPos4K, tab_pt.offset(index as int))
		}
	1 => {
		assert!(index < 512*512*512)
		PTE::new(PTEPos2M, tab_pd.offset(index as int))
		},
	2 => {
		assert!(index < 512*512)
		PTE::new(PTEPos1G, tab_pdp.offset(index as int))
		},
	3 => {
		assert!(index < 512)
		PTE::new(PTEPos512G, tab_pml4.offset(index as int))
		},
	_ => fail!("Passed invalid number to get_entry, {} > 3", level)
	}
}
unsafe fn get_page_ent(addr: uint, from_temp: bool, allocate: bool, large_ok: bool) -> PTE
{
	//log_trace!("get_page_ent(addr={:#x}, from_temp={}, allocate={}", addr, from_temp, allocate);
	let pagenum = (addr & MASK_VBITS) / ::PAGE_SIZE;
	//log_trace!("pagenum = {:#x}", pagenum);
	let mut ent = get_entry(3, pagenum >> (9*3));
	//log_trace!("ent(3) = {}", ent);
	// 1. Walk down page tables from PML4
	if !ent.is_present() {
		//log_trace!("Not present");
		return PTE::null();
	}
	ent = get_entry(2, pagenum >> (9*2));
	//log_trace!("ent(2) = {}", ent);
	if !ent.is_present() {
		return PTE::null();
	}
	if ent.is_large() {
		fail!("TODO: Support large pages (1GiB)");
	}
	ent = get_entry(1, pagenum >> (9*1));
	//log_trace!("ent(1) = {}", ent);
	if !ent.is_present() {
		return PTE::null();
	}
	if ent.is_large() && large_ok {
		return ent;
	}
	return get_entry(0, pagenum)
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
		pte.set_addr( phys );
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
		PTE { pos: PTEPosAbsent, data: RawPtr::null() }
	}

	pub fn is_null(&self) -> bool { self.pos == PTEPosAbsent }
	pub unsafe fn is_reserved(&self) -> bool { !self.is_null() && *self.data != 0 }
	pub unsafe fn is_present(&self) -> bool { !self.is_null() && *self.data & 1 != 0 }
	pub unsafe fn is_large(&self) -> bool { *self.data & (PF_PRESENT | PF_LARGE) == PF_LARGE|PF_PRESENT }
	
	pub unsafe fn set_addr(&self, paddr: PAddr) { *self.data = (*self.data & 0x7FFFFFFF_FFFFF000) | paddr; }
}

impl ::core::fmt::Show for PTE
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		unsafe { write!(f, "PTE({}, *{}={:#x})", self.pos, self.data, *self.data) }
	}
}

// vim: ft=rust

