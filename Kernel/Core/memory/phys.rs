// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/phys.rs
// - Physical memory manager
#[allow(unused_imports)]
use prelude::*;
//use arch::memory::addresses::{physinfo_start, physinfo_end};
use arch::memory::PAddr;

pub const NOPAGE : PAddr = 1;

static S_MEM_MAP: ::lib::LazyStatic<&'static [::memory::MemoryMapEnt]> = lazystatic_init!();
/// Tracks the allocation point in S_MEM_MAP : (Entry Index, Address)
static S_MAPALLOC : ::sync::Mutex<(usize,PAddr)> = mutex_init!( (0,0) );
// TODO: Multiple stacks based on page colouring
static S_FREE_STACK : ::sync::Mutex<PAddr> = mutex_init!( NOPAGE );
// TODO: Reference counts (maybe require arch to expose that)

/// A handle to a physical page (maintaining a reference to it, even when not mapped)
pub struct FrameHandle(PAddr);

pub fn init()
{
	// 1. Acquire a memory map from the architecture code and save for use later
	// SAFE: Called in a true single-threaded context
	unsafe {
		S_MEM_MAP.prep(|| ::arch::boot::get_memory_map());
	}
	
	log_log!("Memory Map:");
	let map = get_memory_map();
	if map.len() == 0 {
		panic!("Empty memory map! Physical memory manager cannot operate");
	}
	for (i,ent) in map.iter().enumerate()
	{
		log_log!("#{} : {:?}", i, ent);
	}
	let mut i = 0;
	while i != map.len() && map[i].state != ::memory::memorymap::MemoryState::Free {
		i += 1;
	}
	if i == map.len() {
		panic!("No free memory in map.");
	}
	*S_MAPALLOC.lock() = (i, map[i].start as PAddr);
}

impl FrameHandle
{
	/// UNSAFE due to using a raw physical address
	pub unsafe fn from_addr(addr: PAddr) -> FrameHandle {
		mark_used(addr);
		FrameHandle(addr)
	}
	/// UNSAFE due to using a raw physical address, and can cause a leak
	pub unsafe fn from_addr_noref(addr: PAddr) -> FrameHandle {
		FrameHandle(addr)
	}
	pub fn into_addr(self) -> PAddr {
		self.0
	}
}

fn get_memory_map() -> &'static [::memory::MemoryMapEnt]
{
	&*S_MEM_MAP
}

fn is_ram(phys: PAddr) -> bool
{
	for e in S_MEM_MAP.iter()
	{
		if e.start as PAddr <= phys && phys < (e.start + e.size) as PAddr
		{
			return match e.state
				{
				::memory::memorymap::MemoryState::Free => true,
				::memory::memorymap::MemoryState::Used => true,
				_ => false,
				};
		}
	}
	false
}

pub fn make_unique(page: PAddr, virt_addr: &[u8; 0x1000]) -> PAddr
{
	if !is_ram(page) {
		panic!("Calling 'make_unique' on non-RAM page");
	}
	else if ::arch::memory::phys::get_multiref_count(page as u64 / ::PAGE_SIZE as u64) == 0 {
		page
	}
	else {
		// 1. Allocate a new frame in temp region
		let mut new_frame = ::memory::virt::alloc_free().expect("TODO: handle OOM in make_unique");
		// 2. Copy in content of old frame
		// SAFE: Both are arrays of 0x1000 bytes
		unsafe { ::core::ptr::copy_nonoverlapping(&virt_addr[0], &mut new_frame[0], 0x1000); }
		new_frame.into_frame().into_addr()
	}
}

pub fn allocate_range_bits(bits: u8, count: usize) -> PAddr
{
	// XXX: HACK! Falls back to the simple code if possible
	if get_memory_map().last().unwrap().start >> bits == 0
	{
		return allocate_range(count);
	}
	// 1. Locate the last block of a suitable bitness
	// - Take care to correctly handle blocks that straddle bitness boundaries
	// NOTE: Memory map constructor _can_ break blocks up at common bitness boundaries (16, 24, 32 bits) to make this more efficient
	// 2. Obtain `count` pages from either the end (if possible) or the start of this block
	// TODO: If the block is not large enough, return an error (NOPAGE)
	panic!("TODO: allocate_range(bits={}, count={})", bits, count);
}

pub fn allocate_range(count: usize) -> PAddr
{
	let mut h = S_MAPALLOC.lock();
	log_trace!("allocate_range: *h = ({},{:#x}) (init)", h.0, h.1);
	let (mut i,mut addr) = *h;
	
	let map = get_memory_map();
	if i == map.len() {
		log_error!("Out of physical memory");
		return NOPAGE;
	}
	if addr + ::PAGE_SIZE as PAddr > map[i].end() as PAddr
	{
		i += 1;
		while i != map.len() && map[i].state != ::memory::memorymap::MemoryState::Free {
			i += 1;
		}
		if i == map.len() {
			log_error!("Out of physical memory");
			*h = (i, 0);
			return NOPAGE;
		}
		addr = map[i].start as PAddr;
	}
	let rv = addr;
	let shift = (count * ::PAGE_SIZE) as PAddr;
	if addr + shift > map[i].end() as PAddr {
		todo!("Handle allocating from ahead in map ({:#x} + {:#x} > {:#x})", addr, shift, map[i].end());
	}
	addr += shift;
	//log_trace!("allocate_range: rv={:#x}, i={}, addr={:#x}", rv, i, addr);
	*h = (i, addr);
	//log_trace!("allocate_range: *h = {:?}", *h);
	return rv;
}

pub fn allocate_bare() -> Result<PAddr, ()> {
	allocate_int(None)
}

pub fn allocate(address: *mut ()) -> bool {
	allocate_int(Some(address)).is_ok()
}

fn allocate_int( address: Option<*mut ()> ) -> Result<PAddr, ()>
{
	log_trace!("allocate(address={:?})", address);
	// 1. Pop a page from the free stack
	// SAFE: Frames on the free are not aliased, alloc is safe
	unsafe
	{
		let mut h = S_FREE_STACK.lock();
		let paddr = *h;
		if paddr != NOPAGE
		{
			// If calling map on this address will not cause a recursive allocation
			match address
			{
			Some(address) => {
				if ::arch::memory::virt::can_map_without_alloc(address) {
					// Map and obtain the next page
					::memory::virt::map(address, paddr, super::virt::ProtectionMode::KernelRW);
					*h = *(address as *const PAddr);
				}
				else {
					// Otherwise, do a temp mapping, extract the next page, then drop the lock and map
					// NOTE: A race here doesn't matter, as lower operations are atomic, and it'd just be slower
					::memory::virt::with_temp(paddr, |page| *h = *(&page[0] as *const u8 as *const PAddr));
					drop(h);
					::memory::virt::map(address, paddr, super::virt::ProtectionMode::KernelRW);
				}
				//*(address as *mut PAddr) = 0;
				*(address as *mut [u8; ::PAGE_SIZE]) = ::core::mem::zeroed();
				log_trace!("- {:p} (stack) paddr = {:#x}", address, paddr);
				},
			None => {
				::memory::virt::with_temp(paddr, |page| *h = *(&page[0] as *const u8 as *const PAddr));
				log_trace!("- None (stack) paddr = {:#x}", paddr);
				},
			}
			mark_used(paddr);
			return Ok(paddr);
		}
	}
	// 2. If none, allocate from map
	let paddr = allocate_range(1);
	if paddr != NOPAGE
	{
		if let Some(address) = address {
			// SAFE: Physical address just allocated
			unsafe {
				::memory::virt::map(address, paddr, super::virt::ProtectionMode::KernelRW);
				*(address as *mut [u8; ::PAGE_SIZE]) = ::core::mem::zeroed();
			}
			log_trace!("- {:p} (range) paddr = {:#x}", address, paddr);
		}
		else {
			log_trace!("- None (range) paddr = {:#x}", paddr);
		}
		return Ok(paddr);
	}
	// 3. Fail
	log_trace!("- (none)");
	Err( () )
}

pub fn ref_frame(paddr: PAddr)
{
	if ! is_ram(paddr) {
		
	}
	else {
		::arch::memory::phys::ref_frame(paddr as u64 / ::PAGE_SIZE as u64);
	}
}
pub fn deref_frame(paddr: PAddr)
{
	if ! is_ram(paddr) {
		log_log!("Calling deref_frame on non-RAM {:#x}", paddr);
	}
	// Dereference page (returns prevous value, zero meaning page was not multi-referenced)
	else if ::arch::memory::phys::deref_frame(paddr as u64 / ::PAGE_SIZE as u64) == 0 {
		// - This page is the only reference.
		if ::arch::memory::phys::mark_free(paddr as u64 / ::PAGE_SIZE as u64) == true {
			// Release frame back into the pool
			// SAFE: This frame is unaliased
			unsafe {
				let mut h = S_FREE_STACK.lock();
				::memory::virt::with_temp(paddr, |page| *(&mut page[0] as *mut u8 as *mut PAddr) = *h);
				*h = paddr;
			}
		}
		else {
			// Page was either not allocated (oops) or is not managed
			// - Either way, ignore
		}
	}
}

fn mark_used(paddr: PAddr)
{
	log_error!("TODO: mark_used(paddr={:#x})", paddr);
	// TODO:
}

// vim: ft=rust
