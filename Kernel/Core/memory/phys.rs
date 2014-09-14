// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/phys.rs
// - Physical memory manager
use _common::*;
use arch::memory::addresses::{physinfo_start, physinfo_end};
use arch::memory::{PAddr,VAddr};

static NOPAGE : PAddr = 0;	// Frame 0 is reserved for system use, and should never reach the stack

static mut s_mem_map : Option<&'static [::memory::MemoryMapEnt]> = None;
static mut s_mapalloc : ::sync::Mutex<PAddr> = mutex_init!( 0 );
// TODO: Multiple stacks based on page colouring
static mut s_free_stack : ::sync::Mutex<PAddr> = mutex_init!( NOPAGE );

pub fn init()
{
	// 1. Acquire a memory map from the architecture code
	unsafe {
		s_mem_map = Some(::arch::boot::get_memory_map());
	}
	// 2. Save the map for use with allocation functions
}

pub fn allocate_range(count: uint) -> PAddr
{
	unsafe
	{
		let h = s_mapalloc.lock();
		fail!("TODO: ::memory::phys::allocate_range(count={})", count);
		NOPAGE
	}
}

pub fn allocate(address: *mut ()) -> bool
{
	// 1. Pop a page from the free stack
	unsafe
	{
		let mut h = s_free_stack.lock();
		let paddr = *h;
		if paddr != NOPAGE
		{
			::memory::virt::map(address, paddr, super::virt::ProtKernelRO);
			*h = *(address as *const PAddr);
			mark_used(paddr);
			return true;
		}
	}
	// 2. If none, allocate from map
	let paddr = allocate_range(1);
	if paddr != NOPAGE
	{
		::memory::virt::map(address, paddr, super::virt::ProtKernelRO);
		return true
	}
	// 3. Fail
	false
}

fn mark_used(paddr: PAddr)
{
	// TODO:
}

// vim: ft=rust
