// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/phys.rs
// - Physical memory manager
use _common::*;
//use arch::memory::addresses::{physinfo_start, physinfo_end};
use arch::memory::PAddr;
use lib::LazyStatic;

pub const NOPAGE : PAddr = 1;

static mut s_mem_map : Option<&'static [::memory::MemoryMapEnt]> = None;
// s_mapalloc - Tracks the allocation point in s_mem_map : (Entry Index, Address)
#[allow(non_upper_case_globals)]
static s_mapalloc : ::sync::Mutex<(usize,PAddr)> = mutex_init!( (0,0) );
// TODO: Multiple stacks based on page colouring
#[allow(non_upper_case_globals)]
static s_free_stack : ::sync::Mutex<PAddr> = mutex_init!( NOPAGE );

pub fn init()
{
	// 1. Acquire a memory map from the architecture code and save for use later
	unsafe {
		s_mem_map = Some( ::arch::boot::get_memory_map() );
	}
	
	log_log!("Memory Map:");
	for (i,ent) in get_memory_map().iter().enumerate()
	{
		log_log!("#{} : {:?}", i, ent);
	}
}

fn get_memory_map() -> &'static [::memory::MemoryMapEnt]
{
	unsafe {
		s_mem_map.unwrap()
	}
}

pub fn allocate_range_bits(bits: u8, count: usize) -> PAddr
{
	// XXX: HACK! Falls back to the simple code if possible
	if count == 1 && get_memory_map().last().unwrap().start >> bits == 0
	{
		return allocate_range(1);
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
	if !(count == 1) {
		panic!("TODO: Large range allocations (count={})", count);
	}

	let mut h = s_mapalloc.lock();
	//log_trace!("allocate_range: *h = {:?} (init)", *h);
	let (mut i,mut addr) = *h;
	
	let map = get_memory_map();
	if i == map.len() {
		log_error!("Out of physical memory");
		return NOPAGE;
	}
	if addr >= map[i].start + map[i].size
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
		addr = map[i].start;
	}
	let rv = addr;
	addr += ::PAGE_SIZE as u64;
	//log_trace!("allocate_range: rv={:#x}, i={}, addr={:#x}", rv, i, addr);
	*h = (i, addr);
	//log_trace!("allocate_range: *h = {:?}", *h);
	return rv;
}

pub fn allocate(address: *mut ()) -> bool
{
	log_trace!("allocate(address={:p})", address);
	// 1. Pop a page from the free stack
	unsafe
	{
		let mut h = s_free_stack.lock();
		let paddr = *h;
		if paddr != NOPAGE
		{
			::memory::virt::map(address, paddr, super::virt::ProtectionMode::KernelRO);
			*h = *(address as *const PAddr);
			*(address as *mut [u8; ::PAGE_SIZE]) = ::core::mem::zeroed();
			mark_used(paddr);
			log_trace!("- {:p} (stack) paddr = {:#x}", address, paddr);
			return true;
		}
	}
	// 2. If none, allocate from map
	let paddr = allocate_range(1);
	if paddr != NOPAGE
	{
		::memory::virt::map(address, paddr, super::virt::ProtectionMode::KernelRW);
		unsafe { *(address as *mut [u8; ::PAGE_SIZE]) = ::core::mem::zeroed(); }
		log_trace!("- {:p} (range) paddr = {:#x}", address, paddr);
		return true
	}
	// 3. Fail
	log_trace!("- (none)");
	false
}

fn mark_used(paddr: PAddr)
{
	log_error!("TODO: mark_used(paddr={:#x})", paddr);
	// TODO:
}

// vim: ft=rust
