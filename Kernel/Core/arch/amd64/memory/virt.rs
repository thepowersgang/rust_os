// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/memory/virt.rs
//! Virtual address space management
use super::PAddr;
use crate::PAGE_SIZE;
use crate::memory::virt::{ProtectionMode,MapError};
use crate::arch::memory::virt::TempHandle;
use crate::arch::memory::PAGE_MASK;
use super::addresses;

const MASK_VBITS : usize = 0x0000FFFF_FFFFFFFF;

const FLAG_P:   u64 = 1;
const FLAG_W:   u64 = 2;
const FLAG_U:   u64 = 4;
const FLAG_G:   u64 = 0x100;
const FLAG_COW: u64 = 0x200;	// free bit, overloaded as COW
const FLAG_NX:  u64 = 1<<63;

const FAULT_LOCKED: u32 = 1;
const FAULT_WRITE:  u32 = 2;
const FAULT_USER:   u32 = 4;
const FAULT_RESVD:  u32 = 8;
const FAULT_FETCH:  u32 = 16;

extern "C" {
	static InitialPML4: [u64; 512];
}

pub fn post_init() {
	// TODO: Clear initial mapping
}


#[derive(PartialEq,Debug)]
enum PTEPos
{
	Absent,
	Page512G,
	Page1G,
	Page2M,
	Page4K,
}
impl PTEPos {
	fn from_level(level: u8) -> PTEPos {
		match level {
		0 => PTEPos::Absent,
		1 => PTEPos::Page4K,
		2 => PTEPos::Page2M,
		3 => PTEPos::Page1G,
		4 => PTEPos::Page512G,
		_ => panic!("Invalid level {} for PTEPos::from_level", level),
		}
	}
}

struct PTE
{
	pos: PTEPos,
	data: *mut u64
}

enum LargeOk { Yes, No }
impl LargeOk { fn yes(&self) -> bool { match self { &LargeOk::Yes => true, _ => false } } }

/// UNSAFE:
/// - Provides possibly-aliased &mut-s (upper code should handle aliasing concerns by using locks)
/// - Regions covered by returned pointers may not be mapped
unsafe fn get_tables<'a>() -> (&'a mut [u64; 512<<3*9], &'a mut [u64; 512<<2*9], &'a mut [u64; 512<<9], &'a mut [u64; 512])
{
	let pt_page = (addresses::FRACTAL_BASE & MASK_VBITS) / PAGE_SIZE;
	let tab_pt = addresses::FRACTAL_BASE as *mut u64;
	let tab_pd = tab_pt.offset( pt_page as isize );
	let tab_pdp = tab_pd.offset( (pt_page >> (9)) as isize );
	let tab_pml4 = tab_pdp.offset( (pt_page >> (9+9)) as isize );
	//log_debug!("tab_pt = {:p}, tab_pd = {:p}, tab_pdp = {:p}, tab_pml4 = {:p}",
	//	tab_pt, tab_pd, tab_pdp, tab_pml4);
	(&mut *(tab_pt as *mut _), &mut *(tab_pd as *mut _), &mut *(tab_pdp as *mut _), &mut *(tab_pml4 as *mut _))
}

/// Get a page entry given the desired level and index
///
/// UNSAFE: Doesn't (and can't) check if `index` points into an allocated parent table
unsafe fn get_entry(level: u8, index: usize, force_allocate: bool) -> PTE
{
	let index = if index < (1<<(4*9)) { index } else { panic!("{} index OOR {}", module_path!(), index) };
	
	let (tab_pt, tab_pd, tab_pdp, tab_pml4) = get_tables();
	
	//log_trace!("get_entry(level={}, index={:#x})", level, index);
	// NOTE: Does no checks on presence
	match level
	{
	0 => {
		assert!(index < 512*512*512*512);
		assert!(tab_pml4[index >> 27] & 1 != 0);
		assert!(tab_pdp[index >> 18] & 1 != 0);
		assert!(tab_pd[index >> 9] & 1 != 0);
		PTE::new(PTEPos::Page4K, &mut tab_pt[index as usize])
		}
	1 => {
		const MAX_IDX: usize = 512*512*512;
		assert!(index < MAX_IDX);
		assert!(tab_pml4[index >> 18] & 1 != 0);
		assert!(tab_pdp[index >> 9] & 1 != 0);
		let rv = PTE::new(PTEPos::Page2M, &mut tab_pd[index]);
		if !rv.is_present() && force_allocate {
			let ptr = &mut tab_pt[index * 512] as *mut u64 as *mut ();
			//log_debug!("Allocating for {:?} (PD Ent {})", ptr, index);
			crate::memory::phys::allocate( ptr );
			// - If manipulating the user's half of the address space, allow them access (full permissions)
			if index < MAX_IDX/2 {
				reprotect(ptr, ProtectionMode::UserRWX);
			}
		}
		rv
		},
	2 => {
		const MAX_IDX: usize = 512*512;
		assert!(index < MAX_IDX);
		assert!(tab_pml4[index >> 9] & 1 != 0);
		let rv = PTE::new(PTEPos::Page1G, &mut tab_pdp[index]);
		if !rv.is_present() && force_allocate {
			let ptr = &mut tab_pd[index * 512] as *mut u64 as *mut ();
			//log_debug!("Allocating for {:?} (PDPT Ent {})", ptr, index);
			crate::memory::phys::allocate( ptr );
			if index < MAX_IDX/2 {
				reprotect(ptr, ProtectionMode::UserRWX);
			}
		}
		rv
		},
	3 => {
		const MAX_IDX: usize = 512;
		assert!(index < MAX_IDX);
		let rv = PTE::new(PTEPos::Page512G, &mut tab_pml4[index]);
		if !rv.is_present() && force_allocate {
			let ptr = &mut tab_pdp[index * 512] as *mut u64 as *mut ();
			//log_debug!("Allocating for {:?} (PML4 Ent {})", ptr, index);
			crate::memory::phys::allocate( ptr );
			if index < MAX_IDX/2 {
				reprotect(ptr, ProtectionMode::UserRWX);
			}
		}
		rv
		},
	_ => panic!("Passed invalid number to get_entry, {} > 3", level)
	}
}

fn get_page_ent(addr: usize, allocate: bool, large_ok: LargeOk) -> PTE
{
	let pagenum = (addr & MASK_VBITS) / PAGE_SIZE;
	//log_trace!("get_page_ent(addr={:#x}, from_temp={}, allocate={}), pagenum={:#x}", addr, from_temp, allocate, pagenum);

	// SAFE: Calls 'get_entry' down the tree to ensure validity
	unsafe {
		let ent = get_entry(3, pagenum >> (9*3), allocate );
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
			if large_ok.yes() {
				return ent;
			}
			else {	
				return PTE::null();
			}
		}

		get_entry(0, pagenum, allocate)
	}
}

/// Returns Some(addr) if the passed physical address is in a fixed allocation range (i.e. kernel's identity range)
pub fn fixed_alloc(addr: PAddr, page_count: usize) -> Option<*mut ()>
{
	const FOURMEG: PAddr = (addresses::IDENT_END - addresses::IDENT_START) as PAddr;
	if addr < FOURMEG && addr + (page_count * crate::PAGE_SIZE) as PAddr <= FOURMEG
	{
		Some( (addresses::IDENT_START + addr as usize) as *mut () )
	}
	else
	{
		None
	}
}

/// Returns true if the passed virtual address is within the fixed allocation region
pub fn is_fixed_alloc(addr: *const (), page_count: usize) -> bool
{
	use super::addresses::{IDENT_START,IDENT_END};
	
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
	let pte = get_page_ent(addr as usize, false, LargeOk::Yes);
	return !pte.is_null() && pte.is_reserved();
}
/// Returns the physical address for the provided pointer
pub fn get_phys<T>(addr: *const T) -> PAddr
{
	let pte = get_page_ent(addr as usize, false, LargeOk::Yes);
	if pte.is_large() {
		pte.addr() + ((addr as usize) & 0x1FFFFF) as u64
	}
	else {
		pte.addr() + ((addr as usize) & 0xFFF) as u64
	}
}
pub fn get_info<T>(addr: *const T) -> Option<(PAddr,ProtectionMode)>
{
	let pte = get_page_ent(addr as usize, false, LargeOk::Yes);
	if pte.is_reserved() {
		Some( (pte.addr(), pte.get_perms()) )
	}
	else {
		None
	}
}

fn invlpg(addr: *mut ()) {
	// SAFE: Cannot cause memory unsafety
	unsafe {
		::core::arch::asm!("invlpg [{}]", in(reg) addr);
	}
}

pub fn can_map_without_alloc(addr: *mut ()) -> bool {
	// The following only returns PTE::null() if an intermediate step was unallocated
	! get_page_ent(addr as usize, false, LargeOk::No).is_null()
}

/// Maps a physical frame to a page, with the provided protection mode
pub unsafe fn map(addr: *mut (), phys: PAddr, prot: crate::memory::virt::ProtectionMode)
{
	let mut pte = get_page_ent(addr as usize, true, LargeOk::No);
	assert!( !pte.is_null(), "Failed to obtain ent for {:p}", addr );
	if pte.set_if_unset( phys, prot ).is_err() {
		panic!("Attempting to map over existing allocation addr={:p}", addr);
	}
	invlpg(addr);
}
/// Removes a mapping
pub unsafe fn unmap(addr: *mut ()) -> Option<PAddr>
{
	let mut pte = get_page_ent(addr as usize, false, LargeOk::No);
	assert!( !pte.is_null(), "Failed to obtain ent for {:p}", addr );
	let rv = if pte.is_present() {
			Some(pte.addr())
		}
		else {
			None
		};
	pte.set( 0, crate::memory::virt::ProtectionMode::Unmapped );
	
	invlpg(addr);
	
	rv
}
/// Change protections mode
pub unsafe fn reprotect(addr: *mut (), prot: crate::memory::virt::ProtectionMode)
{
	assert!( !is!(prot, crate::memory::virt::ProtectionMode::Unmapped) );
	let mut pte = get_page_ent(addr as usize, true, LargeOk::No);
	assert!( !pte.is_null(), "Failed to obtain ent for {:p}", addr );
	assert!( pte.is_present(), "Reprotecting unmapped page {:p}", addr );
	let phys = pte.addr();
	pte.set( phys, prot );
	invlpg(addr);
}

static PF_PRESENT : u64 = 0x001;
static PF_LARGE   : u64 = 0x080;

impl PTE
{
	// UNSAFE: Ensure that this pointer is unique and valid.
	pub unsafe fn new(pos: PTEPos, ptr: *mut u64) -> PTE {
		PTE { pos: pos, data: ptr }
	}
	pub fn null() -> PTE {
		PTE { pos: PTEPos::Absent, data: ::core::ptr::null_mut() }
	}

	pub fn is_null(&self) -> bool {
		self.pos == PTEPos::Absent
	}
	pub fn is_reserved(&self) -> bool {
		// SAFE: Construction should ensure this pointer is valid
		unsafe {
			!self.is_null() && *self.data != 0
		}
	}
	pub fn is_present(&self) -> bool {
		// SAFE: Construction should ensure this pointer is valid
		unsafe {
			!self.is_null() && *self.data & 1 != 0
		}
	}
	pub fn is_large(&self) -> bool {
		// SAFE: Construction should ensure this pointer is valid
		unsafe {
			self.is_present() && *self.data & (PF_PRESENT | PF_LARGE) == PF_LARGE|PF_PRESENT
		}
	}
	pub fn is_cow(&self) -> bool {
		// SAFE: Construction should ensure this pointer is valid
		unsafe {
			self.is_present() && (*self.data & FLAG_COW != 0)
		}
	}
	
	pub fn addr(&self) -> PAddr {
		// SAFE: Construction should ensure this pointer is valid
		unsafe {
			*self.data & 0x7FFFFFFF_FFFFF000
		}
	}
	//pub unsafe fn set_addr(&self, paddr: PAddr) {
	//	assert!(!self.is_null());
	//	*self.data = (*self.data & !0x7FFFFFFF_FFFFF000) | paddr;
	//}
	
	pub fn mode_to_flags(prot: crate::memory::virt::ProtectionMode) -> u64 {
		match prot
		{
		ProtectionMode::Unmapped => 0,
		ProtectionMode::KernelRO => FLAG_P|FLAG_NX,
		ProtectionMode::KernelRW => FLAG_P|FLAG_NX|FLAG_W,
		ProtectionMode::KernelRX => FLAG_P,
		ProtectionMode::UserRO   => FLAG_P|FLAG_U|FLAG_NX,
		ProtectionMode::UserRW   => FLAG_P|FLAG_U|FLAG_NX|FLAG_W,
		ProtectionMode::UserCOW  => FLAG_P|FLAG_U|FLAG_NX|FLAG_COW,
		ProtectionMode::UserRX   => FLAG_P|FLAG_U,
		ProtectionMode::UserRWX  => FLAG_P|FLAG_U|FLAG_W,
		}
	}

	// UNSAFE: Can invaidate virtual addresses and cause aliasing
	pub unsafe fn set(&mut self, paddr: PAddr, prot: crate::memory::virt::ProtectionMode) {
		assert!(!self.is_null());
		let flags: u64 = Self::mode_to_flags(prot);
		*self.data = (paddr & 0x7FFFFFFF_FFFFF000) | flags;
	}
	
	pub fn set_if_unset(&mut self, paddr: PAddr, prot: crate::memory::virt::ProtectionMode) -> Result<(),()> {
		assert!(!self.is_null());
		let flags: u64 = Self::mode_to_flags(prot);
		let v = (paddr & 0x7FFFFFFF_FFFFF000) | flags;
		// SAFE: Atomic 64-bit and valid pointer
		if unsafe { ::core::intrinsics::atomic_cxchg_relaxed(self.data, 0, v).0 } == 0 {
			Ok( () )
		}
		else {
			Err( () )
		}
	}
	
	pub fn get_perms(&self) -> crate::memory::virt::ProtectionMode {
		assert!(!self.is_null());
		// SAFE: Pointer should be valid
		let val = unsafe { *self.data };
		if val & FLAG_P == 0 {
			ProtectionMode::Unmapped
		}
		else {
			let flags = val & (FLAG_U|FLAG_NX|FLAG_W);
			const U_RX : u64 = FLAG_U;
			const U_RWX: u64 = FLAG_U|FLAG_W;
			const U_RO : u64 = FLAG_U|FLAG_NX;
			const U_RW : u64 = FLAG_U|FLAG_W|FLAG_NX;
			match flags
			{
			0 => ProtectionMode::KernelRX,
			//0|FLAG_W => ProtectionMode::KernelRWX,
			U_RX  => ProtectionMode::UserRX,
			U_RWX => ProtectionMode::UserRWX,
			U_RO  => if val & FLAG_COW != 0 { ProtectionMode::UserCOW } else { ProtectionMode::UserRO },
			U_RW  => ProtectionMode::UserRW,
			_ => todo!("PTE::get_perms() - Todo {:#x}", flags),
			}
		}
	}
}

impl ::core::fmt::Debug for PTE
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		// SAFE: Pointer is either NULL or valid
		let val = unsafe { if self.is_null() { 0 } else { *self.data } };
		
		let addr = val & !(FLAG_NX|0xFFF);
		write!(f,
			"PTE({:?}, {:#x} = {:#x} [{}{}{}{}{}])",
			self.pos, val, addr,
			if val & FLAG_G != 0   { "g" } else { "" },
			if val & FLAG_U != 0   { "u" } else { "" },
			if val & FLAG_W != 0   { "w" } else { "" },
			if val & FLAG_NX != 0  { "" } else { "x" },
			if val & FLAG_COW != 0 { "C" } else { "" },
			)
	}
}

/// Handle a page fault in whatever way is suitable
pub fn handle_page_fault(accessed_address: usize, error_code: u32) -> bool
{
	// Check clobbered bits first
	if error_code & FAULT_RESVD != 0 {
		// Reserved bits of the page directory were clobbered, this is a kernel panic
		panic!("Reserved bits clobbered {:#x}", accessed_address);
	}
	
	let mut pte = get_page_ent(accessed_address, false, LargeOk::Yes);
	
	// - Global rules
	//  > Copy-on-write pages
	if error_code & (FAULT_WRITE|FAULT_LOCKED) == (FAULT_WRITE|FAULT_LOCKED) && pte.is_cow() {
		// Poke the main VMM layer
		//::memory::virt::cow_write(accessed_address);

		// 1. Lock (relevant) address space
		// SAFE: Changes to address space are transparent
		crate::memory::virt::with_lock(accessed_address, || unsafe {
			let frame = pte.addr();
			let pgaddr = (accessed_address as usize) & !PAGE_MASK;
			// 2. Get the PMM to provide us with a unique copy of that frame (can return the same addr)
			let newframe = crate::memory::phys::make_unique( frame, &*(pgaddr as *const [u8; 4096]) );
			// 3. Remap to this page as UserRW (because COW is user-only atm)
			pte.set(newframe, ProtectionMode::UserRW);
			invlpg( (accessed_address & !0xFFF) as *mut () );
			});
		return true;
	}
	//  > Paged-out pages
	if error_code & FAULT_LOCKED == 0 && pte.is_reserved() {
		todo!("Paged - {:#x} pte = {:?}", accessed_address, pte);
	}
	
	
	// Check if the user is buggy
	if error_code & FAULT_USER != 0 {
		log_log!("User {} {} memory{} : {:#x}",
			if error_code & FAULT_WRITE  != 0 { "write to"  } else { "read from" },
			if error_code & FAULT_LOCKED != 0 { "protected" } else { "non-present" },
			if error_code & FAULT_FETCH != 0 { " (instruction fetch)" } else { "" },
			accessed_address
			);
		return false;
	}
	else {
		log_error!("Kernel {} {} memory{}",
			if error_code & FAULT_WRITE  != 0 { "write to"  } else { "read from" },
			if error_code & FAULT_LOCKED != 0 { "protected" } else { "non-present" },
			if error_code & FAULT_FETCH != 0 { " (instruction fetch)" } else { "" }
			);
		return false;
	}
}


static S_TEMP_FREE: crate::sync::Semaphore = crate::sync::Semaphore::new(NUM_TEMP_SLOTS as isize, NUM_TEMP_SLOTS as isize);
const NUM_TEMP_SLOTS: usize = (addresses::TEMP_END - addresses::TEMP_BASE) / crate::PAGE_SIZE;

pub unsafe fn temp_map<T>(phys: crate::arch::memory::PAddr) -> *mut T {
	S_TEMP_FREE.acquire();

	// 2. Locate a slot
	for i in 0 .. NUM_TEMP_SLOTS {
		let addr = (addresses::TEMP_BASE + i * crate::PAGE_SIZE) as *mut ();

		if get_page_ent(addr as usize, true, LargeOk::No).set_if_unset(phys, ProtectionMode::KernelRW).is_ok() {
			invlpg(addr);
			return addr as *mut T;
		}
	}
	panic!("TempHandle::new() - Semaphore reported free slots, but none found");
}
pub unsafe fn temp_unmap<T>(addr: *mut T) {
	// SAFE: Owned allocation
	/*unsafe*/ {
		get_page_ent(addr as usize, false, LargeOk::No).set(0, ProtectionMode::Unmapped);
		invlpg(addr as *mut ());
	}
	S_TEMP_FREE.release();
}


struct NewTable(TempHandle<u64>);
impl NewTable {
	fn new() -> Result<NewTable,crate::memory::virt::MapError> {
		match crate::memory::phys::allocate_bare()
		{
		Err(crate::memory::phys::Error) => Err( MapError::OutOfMemory ),
		Ok(temp_handle) => Ok( NewTable( temp_handle.into() ) ),
		}
	}
	fn into_frame(self) -> PAddr {
		// SAFE: Forgets after read
		let h = unsafe {
			let h = ::core::ptr::read(&self.0);
			::core::mem::forget(self);
			h
			};
		h.phys_addr()
	}
}
impl ::core::ops::Drop for NewTable {
	fn drop(&mut self) {
		// TODO: This method needs to recursively free paging structures held by it.
		// TODO: Need to also know the level it was used at!
		todo!("NewTable::drop");
	}
}
impl ::core::ops::Deref for NewTable {
	type Target = [u64];
	fn deref(&self) -> &[u64] {
		&self.0
	}
}
impl ::core::ops::DerefMut for NewTable {
	fn deref_mut(&mut self) -> &mut [u64] {
		&mut self.0
	}
}

/// Virtual address space
pub struct AddressSpace(u64);
impl_fmt! {
	Debug(self,f) for AddressSpace { write!(f, "AddressSpace({:#x})", self.0) }
}
impl AddressSpace
{
	pub fn new(clone_start: usize, clone_end: usize) -> Result<AddressSpace,crate::memory::virt::MapError>
	{
		use super::addresses::FRACTAL_BASE;
	
		// Function called when an entry is found to have a table
		fn opt_clone_page(idx: usize) -> Result<u64, crate::memory::virt::MapError>
		{
			//log_trace!("opt_clone_page(idx={:#x})", idx);
			
			// SAFE: Only called when parent table is present
			let ent = unsafe { get_entry(0, idx, false) };
			if ! ent.is_reserved()
			{
				Ok(0)
			}
			else
			{
				let p = ent.get_perms();
				let frame = match p
					{
					ProtectionMode::UserRX | ProtectionMode::UserCOW => {
						let addr = ent.addr();
						crate::memory::phys::ref_frame( addr );
						addr
						},
					ProtectionMode::UserRWX | ProtectionMode::UserRW => {
						// SAFE: We've just determined that this page is mapped in, so we won't crash. Any race is the user's fault (and shouldn't impact the kernel)
						let src = unsafe { ::core::slice::from_raw_parts((idx << 12) as *const u8, PAGE_SIZE) };
						let mut newpg = crate::memory::virt::alloc_free()?;
						for (d,s) in Iterator::zip( newpg.iter_mut(), src.iter() ) {
							*d = *s;
						}
						newpg.into_frame().into_addr()
						},
					v @ _ => todo!("opt_clone_page - Mode {:?}", v),
					};
				Ok( frame | PTE::mode_to_flags(p) )
			}
		}
		fn opt_clone_segment(level: u8, idx: usize, clone_start_pidx: usize, clone_end_pidx: usize) -> Result<u64,crate::memory::virt::MapError>
		{
			//log_trace!("opt_clone_segment(level={}, idx={}, ...)", level, idx);
			
			// SAFE: Only called when parent table is present
			let ent = unsafe { get_entry(level, idx, false) };
			if ! ent.is_reserved()
			{
				Ok(0)
			}
			else if ent.is_large()
			{
				todo!("opt_clone_segment - large page");
			}
			else
			{
				let mut ents = NewTable::new()?;
				let base = idx << 9;
				for i in 0 .. 512
				{
					let this_idx = base + i;
					let level_bits = 9 * (level as usize - 1);
					//log_trace!("{:#x} <= {:#x} && {:#x} < {:#x}", clone_start_pidx >> level_bits, this_idx, this_idx << level_bits, clone_end_pidx);
					if clone_start_pidx >> level_bits <= this_idx && this_idx << level_bits < clone_end_pidx
					{
						ents[i] = if level == 1 {
								opt_clone_page(this_idx)?
							}
							else {
								opt_clone_segment(level-1, this_idx, clone_start_pidx,clone_end_pidx)?
							};
					}
				}
				Ok( ents.into_frame() | FLAG_U|FLAG_W|FLAG_P )
			}
		}
	
		// TODO: Make these two errors
		assert!(clone_start < clone_end);
		assert!(clone_end <= crate::arch::memory::addresses::USER_END);
		
		let clone_start = clone_start >> 12;
		let clone_end = clone_end >> 12;
		
		// - Allocate a new root level
		let mut ents = NewTable::new()?;
		// TODO: Freeze user state during this
		for i in 0 .. 256 {
			const PML4_BITS: usize = 9*3;
			let pdp_base = i << PML4_BITS;
			//log_trace!("{:#x} <= {:#x} && {:#x} < {:#x}", clone_start>>PML4_BITS, i, pdp_base, clone_end);
			if clone_start >> PML4_BITS <= i && pdp_base < clone_end {
				ents[i] = opt_clone_segment(3, i, clone_start,clone_end)?;
			}
		}
		// - Alias in kernel shared pages (pretty much all of them really)
		for i in 256 .. 512 {
			const FRACTAL_IDX: usize = (FRACTAL_BASE & MASK_VBITS) >> 12;
			if i == FRACTAL_IDX >> (9*3) {
				log_debug!("fractal at {}", i);
				ents[i] = get_phys(&ents[0]) | 3;
			}
			else {
				// SAFE: Doesn't change while this is active
				ents[i] = unsafe { InitialPML4[i] };
			}
		}
		log_debug!("ents[..256] = {:#x}", crate::logging::print_iter(ents[..256].iter()));
		Ok( AddressSpace( ents.into_frame() ) )
	}
	pub fn pid0() -> AddressSpace {
		// SAFE: Doesn't change while rust code is active
		AddressSpace( get_phys( unsafe { &InitialPML4 }) )
	}
	
	pub fn get_cr3(&self) -> u64 {
		self.0
	}
}
impl ::core::ops::Drop for AddressSpace {
	fn drop(&mut self) {
		
		fn drop_table_ent(table_ent: &mut u64, level: u8) {
			assert!(1 <= level && level <= 4, "AddressSpace::drop::drop_table_ent - level invalid, {}", level);
			// SAFE: We have &mut
			let pte = unsafe { PTE::new(PTEPos::from_level(level), table_ent) };
			if ! pte.is_reserved() {
				assert!( *table_ent == 0, "TODO: Handle non-zero non-present table entry" );
			}
			else {
				let addr = pte.addr();
				if level == 1 {
					// Level 1, i.e. page table. Just dereference the page
				}
				else {
					// Level 2-4 (PD, PDP, PML4). Recurse
					// SAFE: All paging tables should be uniquely owned, transmute is valid
					unsafe {
						crate::memory::virt::with_temp(addr, |tab_pg| {
							let tab: &mut [u64; 512] = ::core::mem::transmute(tab_pg);
							for e in tab.iter_mut() {
								drop_table_ent(e, level-1);
							}
							});
					}
				}
				// SAFE: Memory no longer referenced
				unsafe {
					crate::memory::phys::deref_frame( addr );
				}
			}
			*table_ent = 0;
		}

		// SAFE: All paging tables should be uniquely owned, transmute is valid
		unsafe {
			crate::memory::virt::with_temp(self.0, |pml4_pg| {
				let pml4: &mut [u64; 512] = ::core::mem::transmute(pml4_pg);
				for e in pml4[..256].iter_mut() {
					drop_table_ent(e, 4);
				}
				});
			crate::memory::phys::deref_frame( self.0 );
		}
	}
}

// vim: ft=rust

