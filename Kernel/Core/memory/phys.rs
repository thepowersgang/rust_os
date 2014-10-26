// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/phys.rs
// - Physical memory manager
use _common::*;
//use arch::memory::addresses::{physinfo_start, physinfo_end};
use arch::memory::PAddr;

const NOPAGE : PAddr = 1;

static mut s_mem_map : Option<&'static [::memory::MemoryMapEnt]> = None;
static mut s_mapalloc : ::sync::Mutex<(uint,PAddr)> = mutex_init!( (0,0) );
// TODO: Multiple stacks based on page colouring
static mut s_free_stack : ::sync::Mutex<PAddr> = mutex_init!( NOPAGE );

pub fn init()
{
	// 1. Acquire a memory map from the architecture code and save for use later
	unsafe {
		s_mem_map = Some(::arch::boot::get_memory_map());
	}
	
	for (i,ent) in unsafe{s_mem_map.unwrap()}.iter().enumerate()
	{
		log_log!("#{} : {}", i, ent);
	}
}

pub fn allocate_range(count: uint) -> PAddr
{
	if !(count == 1) {
		fail!("TODO: Large range allocations (count={})", count);
	}

	unsafe
	{
		let mut h = s_mapalloc.lock();
		let map = s_mem_map.unwrap();
		// 1. Locate the next unused address in the map, start from *h
		let (mut i,mut addr) = *h;
		if i == map.len() {
			log_error!("Out of physical memory")
			return NOPAGE;
		}
		if addr >= map[i].start + map[i].size
		{
			i += 1;
			while i != map.len() && map[i].state != ::memory::memorymap::StateFree {
				i += 1;
			}
			if i == map.len() {
				log_error!("Out of physical memory")
				return NOPAGE;
			}
			addr = map[i].start;
		}
		let rv = addr;
		addr += ::PAGE_SIZE as u64;
		*h = (i, addr);
		return rv;
	}
}

pub fn allocate(address: *mut ()) -> bool
{
	log_trace!("allocate(address={})", address);
	// 1. Pop a page from the free stack
	unsafe
	{
		let mut h = s_free_stack.lock();
		let paddr = *h;
		if paddr != NOPAGE
		{
			::memory::virt::map(address, paddr, super::virt::ProtKernelRO);
			*h = *(address as *const PAddr);
			*(address as *mut [u8,..::PAGE_SIZE]) = ::core::mem::zeroed();
			mark_used(paddr);
			return true;
		}
	}
	// 2. If none, allocate from map
	let paddr = allocate_range(1);
	if paddr != NOPAGE
	{
		::memory::virt::map(address, paddr, super::virt::ProtKernelRW);
		unsafe { *(address as *mut [u8,..::PAGE_SIZE]) = ::core::mem::zeroed(); }
		return true
	}
	// 3. Fail
	false
}

fn mark_used(paddr: PAddr)
{
	log_error!("TODO: mark_used(paddr={:#x})", paddr);
	// TODO:
}

// vim: ft=rust
