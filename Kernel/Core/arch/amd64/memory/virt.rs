// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/memory/virt.rs
//! Virtual address space management
use core::prelude::*;
use super::{PAddr,VAddr};
use PAGE_SIZE;

const MASK_VBITS : usize = 0x0000FFFF_FFFFFFFF;

const FLAG_P:   u64 = 1;
const FLAG_W:   u64 = 2;
const FLAG_U:   u64 = 4;
const FLAG_G:   u64 = 0x100;
const FLAG_COW: u64 = 0x200;	// free bit, overloaded as COW
const FLAG_NX:  u64 = (1<<63);

#[derive(PartialEq,Debug)]
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

unsafe fn get_entry(level: u8, index: usize, force_allocate: bool) -> PTE
{
	use arch::memory::addresses::FRACTAL_BASE;
	let index = if index < (1<<(4*9)) { index as isize } else { panic!("{} index OOR {}", module_path!(), index) };
	
	let pt_page = (FRACTAL_BASE & MASK_VBITS) / PAGE_SIZE;
	let tab_pt = FRACTAL_BASE as *mut u64;
	let tab_pd = tab_pt.offset( pt_page as isize );
	let tab_pdp = tab_pd.offset( (pt_page >> (9)) as isize );
	let tab_pml4 = tab_pdp.offset( (pt_page >> (9+9)) as isize );
	//log_debug!("tab_pt = {}, tab_pd = {}, tab_pdp = {}, tab_pml4 = {}",
	//	tab_pt, tab_pd, tab_pdp, tab_pml4);
	
	//log_trace!("get_entry(level={}, index={:#x})", level, index);
	// NOTE: Does no checks on presence
	match level
	{
	0 => {
		assert!(index < 512*512*512*512);
		PTE::new(PTEPos::Page4K, tab_pt.offset(index))
		}
	1 => {
		assert!(index < 512*512*512);
		let rv = PTE::new(PTEPos::Page2M, tab_pd.offset(index));
		if !rv.is_present() && force_allocate {
			let ptr = tab_pt.offset(index * 512) as *mut ();
			//log_debug!("Allocating for {:?} (PD Ent {})", ptr, index);
			::memory::phys::allocate( ptr );
		}
		rv
		},
	2 => {
		assert!(index < 512*512);
		let rv = PTE::new(PTEPos::Page1G, tab_pdp.offset(index));
		if !rv.is_present() && force_allocate {
			let ptr = tab_pd.offset(index * 512) as *mut ();
			//log_debug!("Allocating for {:?} (PDPT Ent {})", ptr, index);
			::memory::phys::allocate( ptr );
		}
		rv
		},
	3 => {
		assert!(index < 512);
		let rv = PTE::new(PTEPos::Page512G, tab_pml4.offset(index));
		if !rv.is_present() && force_allocate {
			let ptr = tab_pdp.offset(index * 512) as *mut ();
			//log_debug!("Allocating for {:?} (PML4 Ent {})", ptr, index);
			::memory::phys::allocate( ptr );
		}
		rv
		},
	_ => panic!("Passed invalid number to get_entry, {} > 3", level)
	}
}
unsafe fn get_page_ent(addr: usize, from_temp: bool, allocate: bool, large_ok: bool) -> PTE
{
	assert!( from_temp == false );
	let pagenum = (addr & MASK_VBITS) / PAGE_SIZE;
	//log_trace!("get_page_ent(addr={:#x}, from_temp={}, allocate={}), pagenum={:#x}", addr, from_temp, allocate, pagenum);

	let ent = get_entry(3, pagenum >> (9*3), allocate);
	// 1. Walk down page tables from PML4
	if !ent.is_present() {
		//log_trace!("get_page_ent(addr={:#x}, ...) PML4 Ent {} absent", addr, pagenum >> (9*3));
		return PTE::null();
	}

	let ent = get_entry(2, pagenum >> (9*2), allocate);
	if !ent.is_present() {
		//log_trace!("get_page_ent(addr={:#x}, ...) PDPT Ent {} absent", addr, pagenum >> (9*2));
		return PTE::null();
	}
	if ent.is_large() {
		panic!("TODO: Support large pages (1GiB)");
	}

	let ent = get_entry(1, pagenum >> (9*1), allocate);
	if !ent.is_present() {
		//log_trace!("get_page_ent(addr={:#x}, ...) PD Ent {} absent", addr, pagenum >> (9*1));
		return PTE::null();
	}
	if ent.is_large() {
		//log_trace!("Large page covering {:#x}", addr);
		if large_ok {
			return ent;
		}
		else {	
			PTE::null();
		}
	}

	return get_entry(0, pagenum, allocate)
}

/// Returns Some(addr) if the passed physical address is in a fixed allocation range (i.e. kernel's identity range)
pub fn fixed_alloc(addr: PAddr, page_count: usize) -> Option<VAddr>
{
	const FOURMEG: PAddr = (::arch::memory::addresses::IDENT_END - ::arch::memory::addresses::IDENT_START) as PAddr;
	if addr < FOURMEG && (FOURMEG - addr >> 10) as usize > page_count
	{
		Some( ::arch::memory::addresses::IDENT_START + addr as usize )
	}
	else
	{
		None
	}
}

pub fn is_fixed_alloc(addr: *const (), page_count: usize) -> bool
{
	use arch::memory::addresses::{IDENT_START,IDENT_END};
	
	let vaddr = addr as usize;
	if IDENT_START <= vaddr && vaddr < IDENT_END {
		let space = IDENT_END - vaddr;
		assert!(space >> 12 >= page_count);
		true
	}
	else {
		false
	}
}

/// Returns true if the passed address is "valid" (allocated, or delay allocated)
pub fn is_reserved<T>(addr: *const T) -> bool
{
	unsafe {
		let pte = get_page_ent(addr as usize, false, false, true);
		return !pte.is_null() && pte.is_reserved();
	}
}
/// Returns the physical address for the provided pointer
pub fn get_phys<T>(addr: *const T) -> PAddr
{
	unsafe {
		let pte = get_page_ent(addr as usize, false, false, true);
		pte.addr() + ((addr as usize) & 0xFFF) as u64
	}
}
/// Maps a physical frame to a page, with the provided protection mode
pub unsafe fn map(addr: *mut (), phys: PAddr, prot: ::memory::virt::ProtectionMode)
{
	let pte = get_page_ent(addr as usize, false, true, false);
	assert!( !pte.is_null(), "Failed to obtain ent for {:p}", addr );
	pte.set( phys, prot );
	asm!("invlpg ($0)" : : "r" (addr) : "memory" : "volatile");
}
/// Removes a mapping
pub unsafe fn unmap(addr: *mut ())
{
	let pte = get_page_ent(addr as usize, false, false, false);
	pte.set( 0, ::memory::virt::ProtectionMode::Unmapped );
	
	asm!("invlpg ($0)" : : "r" (addr) : "memory" : "volatile");
}
/// Change protections mode
pub unsafe fn reprotect(addr: *mut (), prot: ::memory::virt::ProtectionMode)
{
	assert!( !is!(prot, ::memory::virt::ProtectionMode::Unmapped) );
	let pte = get_page_ent(addr as usize, false, true, false);
	assert!( !pte.is_null(), "Failed to obtain ent for {:p}", addr );
	assert!( pte.is_present(), "Reprotecting unmapped page {:p}", addr );
	let phys = pte.addr();
	pte.set( phys, prot );
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
		PTE { pos: PTEPos::Absent, data: ::core::ptr::null_mut() }
	}

	pub fn is_null(&self) -> bool { self.pos == PTEPos::Absent }
	pub unsafe fn is_reserved(&self) -> bool { !self.is_null() && *self.data != 0 }
	pub unsafe fn is_present(&self) -> bool { !self.is_null() && *self.data & 1 != 0 }
	pub unsafe fn is_large(&self) -> bool { *self.data & (PF_PRESENT | PF_LARGE) == PF_LARGE|PF_PRESENT }
	
	pub unsafe fn addr(&self) -> PAddr { *self.data & 0x7FFFFFFF_FFFFF000 }
	//pub unsafe fn set_addr(&self, paddr: PAddr) {
	//	assert!(!self.is_null());
	//	*self.data = (*self.data & !0x7FFFFFFF_FFFFF000) | paddr;
	//}
	
	pub unsafe fn set(&self, paddr: PAddr, prot: ::memory::virt::ProtectionMode) {
		assert!(!self.is_null());
		let flags: u64 = match prot
			{
			::memory::virt::ProtectionMode::Unmapped => 0,
			::memory::virt::ProtectionMode::KernelRO => FLAG_P|FLAG_NX,
			::memory::virt::ProtectionMode::KernelRW => FLAG_P|FLAG_NX|FLAG_W,
			::memory::virt::ProtectionMode::KernelRX => FLAG_P,
			::memory::virt::ProtectionMode::UserRO => FLAG_P|FLAG_U|FLAG_NX,
			::memory::virt::ProtectionMode::UserRW => FLAG_P|FLAG_U|FLAG_NX|FLAG_W,
			::memory::virt::ProtectionMode::UserCOW=> FLAG_P|FLAG_U|FLAG_NX|FLAG_COW,
			::memory::virt::ProtectionMode::UserRX => FLAG_P|FLAG_U,
			};
		*self.data = (paddr & 0x7FFFFFFF_FFFFF000) | flags;
	}
}

impl ::core::fmt::Debug for PTE
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		unsafe { write!(f, "PTE({:?}, *{:?}={:#x})", self.pos, self.data, *self.data) }
	}
}

// vim: ft=rust

