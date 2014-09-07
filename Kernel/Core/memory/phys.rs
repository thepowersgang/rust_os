// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/phys.rs
// - Physical memory manager

use arch::memory::addresses::{physinfo_start, physinfo_end};

pub type PAddr = u64;
pub type VAddr = uint;

pub fn init()
{
	// 1. Acquire a memory map from the architecture code
	// 2. Scan map and 
	// 2. Save the map for use later on
}

pub fn allocate_range(count: uint) -> PAddr
{
	1
}

pub fn allocate(address: VAddr) -> bool
{
	false
}

// vim: ft=rust
