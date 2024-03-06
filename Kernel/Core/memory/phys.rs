// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/phys.rs
// - Physical memory manager
#[allow(unused_imports)]
use crate::prelude::*;
use crate::arch::memory::PAddr;
use crate::arch::memory::virt::TempHandle;
use crate::memory::phys_track;
use crate::memory::virt;
use crate::PAGE_SIZE;

pub const NOPAGE : PAddr = 1;

pub struct Error;
impl_fmt! {
	Debug(self,f) for Error {
		write!(f, "phys::Error")
	}
}

static S_MEM_MAP: crate::lib::LazyStatic<&'static [crate::memory::MemoryMapEnt]> = lazystatic_init!();
/// Tracks the allocation point in S_MEM_MAP : (Entry Index, Address)
static S_MAPALLOC : crate::sync::Mutex<(usize,PAddr)> = mutex_init!( (0,0) );
// TODO: Multiple stacks based on page colouring
static S_FREE_STACK : crate::sync::Mutex<PAddr> = mutex_init!( NOPAGE );
// TODO: Reference counts (maybe require arch to expose that)

/// A handle to a physical page (maintaining a reference to it, even when not mapped)
pub struct FrameHandle(PAddr);

pub fn init()
{
	// 1. Acquire a memory map from the architecture code and save for use later
	assert!(!S_MEM_MAP.ls_is_valid(), "Double-call of init()");
	S_MEM_MAP.prep(|| crate::arch::boot::get_memory_map());
	
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
	while i != map.len() && map[i].state != crate::memory::memorymap::MemoryState::Free {
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
		//mark_used(addr);
		ref_frame(addr);
		FrameHandle(addr)
	}
	/// UNSAFE due to using a raw physical address, and can cause an alias
	pub unsafe fn from_addr_noref(addr: PAddr) -> FrameHandle {
		FrameHandle(addr)
	}
	pub fn into_addr(self) -> PAddr {
		let rv = self.0;
		::core::mem::forget(self);
		rv
	}
}
impl Clone for FrameHandle
{
	fn clone(&self) -> FrameHandle {
		ref_frame(self.0);
		FrameHandle(self.0)
	}
}
impl Drop for FrameHandle
{
	fn drop(&mut self)
	{
		// SAFE: This handle owns the allocation, and the backing mapping will be freed with the TempHandle free
		unsafe {
			deref_frame(self.0)
		}
	}
}

fn get_memory_map() -> &'static [crate::memory::MemoryMapEnt]
{
	&*S_MEM_MAP
}

fn phys_to_ram_frame(phys: PAddr) -> Option<u64>
{
	for e in S_MEM_MAP.iter()
	{
		if e.start as PAddr <= phys && phys < (e.start + e.size) as PAddr
		{
			let is_ram = match e.state
				{
				crate::memory::memorymap::MemoryState::Free => true,
				crate::memory::memorymap::MemoryState::Used => true,
				_ => false,
				};
			return if is_ram {
					// TODO: Pack RAM (skipping holes)
					Some(phys as u64 / PAGE_SIZE as u64)
				}
				else {
					None
				};
		}
	}
	None
}

// TODO: Why does this take `page` instead of calling `get_phys`
pub fn make_unique(page: PAddr, virt_addr: &[u8; PAGE_SIZE]) -> PAddr
{
	if let Some(frame) = phys_to_ram_frame(page)
	{
		if phys_track::get_multiref_count(frame) == 0 {
			page
		}
		else {
			// 1. Allocate a new frame in temp region
			let mut new_frame = virt::alloc_free().expect("TODO: handle OOM in make_unique");
			// 2. Copy in content of old frame
			new_frame.clone_from_slice( virt_addr );
			new_frame.into_frame().into_addr()
		}
	}
	else
	{
		page
	}
}

pub fn allocate_range_bits(bits: u8, count: usize) -> PAddr
{
	// XXX: HACK! Falls back to the simple code if possible
	if bits >= 64 || get_memory_map().last().unwrap().start >> bits == 0
	{
		return allocate_range(count);
	}
	// 1. Locate the last block of a suitable bitness
	// - Take care to correctly handle blocks that straddle bitness boundaries
	// NOTE: Memory map constructor _can_ break blocks up at common bitness boundaries (16, 24, 32 bits) to make this more efficient
	// 2. Obtain `count` pages from either the end (if possible) or the start of this block
	// TODO: If the block is not large enough, return an error (NOPAGE)
	panic!("TODO: allocate_range_bits(bits={}, count={})", bits, count);
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
	// If there's less than one page left in the map entry, go to the next one
	if addr + (1 * PAGE_SIZE) as PAddr > map[i].end() as PAddr
	{
		i += 1;
		while i != map.len() && map[i].state != crate::memory::memorymap::MemoryState::Free {
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
	let shift = (count * PAGE_SIZE) as PAddr;
	if addr + shift > map[i].end() as PAddr {
		let n_free = (map[i].end() as PAddr - rv) / (PAGE_SIZE as PAddr);
		todo!("Handle allocating from ahead in map ({:#x} + {:#x} > {:#x}, start={:#x}) - nfree={} < count={}", addr, shift, map[i].end(), map[i].start, n_free, count);
		// TODO: If the shift pushes this allocation over the edge of a map entry, stick the remaining entries onto the free stack and move to the next free block
	}
	addr += shift;
	//log_trace!("allocate_range: rv={:#x}, i={}, addr={:#x}", rv, i, addr);
	*h = (i, addr);
	//log_trace!("allocate_range: *h = {:?}", *h);
	return rv;
}

/// Allocate a page with no fixed allocation, returns a temporary handle to it
pub fn allocate_bare() -> Result<TempHandle<u8>, Error> {
	allocate_int(None).map(|x| x.expect("Ok(None) from allocate_int when None passed"))
}

/// Allocate at a given address
pub fn allocate(address: *mut ()) -> bool {
	allocate_int(Some(address)).is_ok()
}

/// Allocate a page at the given (optional) address
/// 
/// If no address is provided, a temporary handle is returned
fn allocate_int( address: Option<*mut ()> ) -> Result<Option<TempHandle<u8>>, Error>
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
			match address
			{
			Some(address) => {
				// Check that calling `virt::map` will not cause us to be called
				if crate::arch::memory::virt::can_map_without_alloc(address) {
					// Map and obtain the next page
					virt::map(address, paddr, super::virt::ProtectionMode::KernelRW);
					*h = *(address as *const PAddr);
				}
				else {
					// Otherwise, do a temp mapping, extract the next page, then drop the lock and map
					// NOTE: A race here doesn't matter, as lower operations are atomic, and it'd just be slower
					virt::with_temp(paddr, |page| *h = *(&page[0] as *const u8 as *const PAddr));
					drop(h);
					virt::map(address, paddr, super::virt::ProtectionMode::KernelRW);
				}
				// Zero page - Why? - Should fill it with dropped :)
				*(address as *mut [u8; PAGE_SIZE]) = ::core::mem::zeroed();
				log_trace!("- {:p} (stack) paddr = {:#x}", address, paddr);
				mark_used(paddr);
				return Ok(None);
				},
			None => {
				let handle = crate::arch::memory::virt::TempHandle::new(paddr);
				*h = *(&handle[0] as *const u8 as *const PAddr);
				log_trace!("- None (stack) paddr = {:#x}", paddr);
				mark_used(paddr);
				return Ok( Some(handle) );
				},
			}
		}
	}
	// 2. If none, allocate from map
	let paddr = allocate_range(1);
	if paddr != NOPAGE
	{
		if let Some(address) = address {
			// SAFE: Physical address just allocated
			unsafe {
				virt::map(address, paddr, super::virt::ProtectionMode::KernelRW);
				*(address as *mut [u8; PAGE_SIZE]) = ::core::mem::zeroed();
			}
			log_trace!("- {:p} (range) paddr = {:#x}", address, paddr);
			mark_used(paddr);
			return Ok( None );
		}
		else {
			log_trace!("- None (range) paddr = {:#x}", paddr);
			mark_used(paddr);
			// SAFE: Physical address was just allocated, can't alias
			let handle = unsafe { crate::arch::memory::virt::TempHandle::new(paddr) };
			return Ok( Some(handle) );
		}
	}
	// 3. Fail
	log_warning!("Out of physical memory");
	Err( Error )
}

pub fn ref_frame(paddr: PAddr)
{
	if let Some(frame) = phys_to_ram_frame(paddr) {
		phys_track::ref_frame(frame);
	}
	else {
		// Ignore non-RAM frame
	}
}
// UNSAFE: This frees memory, so can cause use-after-free
pub unsafe fn deref_frame(paddr: PAddr)
{
	if let Some(frame) = phys_to_ram_frame(paddr) {
		// Dereference page (returns previous value, zero meaning page was not multi-referenced)
		if phys_track::deref_frame(frame) == 0 {
			// - This page is the only reference.
			if phys_track::mark_free(frame) == true {
				// Release frame back into the pool
				// SAFE: This frame is unaliased
				let mut h = S_FREE_STACK.lock();
				virt::with_temp(paddr, |page| *(page.as_mut_ptr() as *mut PAddr) = *h);
				*h = paddr;
			}
			else {
				// Page was either not allocated (oops) or is not managed
				// - Either way, ignore
				// TODO: Should an attempted free of a non-allocated frame trigger a panic/error/warning?
			}
		}
		else {
			// References still exist
		}
	}
	else {
		// NOTE: Don't bother logging, as this can be called when unmapping hardware mappings
		//log_log!("Calling deref_frame on non-RAM {:#x}", paddr);
	}
}

fn mark_used(paddr: PAddr)
{
	// TODO: This causes a double-lock in the PMM (when the `phys_track` module tries to allocate
	if false {
		if let Some(frame) = phys_to_ram_frame(paddr) {
			phys_track::mark_used(frame);
		}
	}
}

// vim: ft=rust
