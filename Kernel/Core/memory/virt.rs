// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/virt.rs
// - Virtual memory manager
use _common::*;

use arch::memory::{PAddr,VAddr};

pub enum ProtectionMode
{	
	ProtUnmapped,	// Inaccessible
	ProtKernelRO,	// Kernel readonly
	ProtKernelRW,	// Kernel read-write
	ProtKernelRX,	// Kernel read-execute
	ProtUserRO,	// User
	ProtUserRW,
	ProtUserRX,
}

#[link_section(process_local)]
static mut s_userspace_lock : ::sync::Mutex<()> = mutex_init!( () );
static mut s_kernelspace_lock : ::sync::Mutex<()> = mutex_init!( () );

pub fn init()
{
	// 1. Tell the architecture-specific VMM that it can clean up init state
	// 2. ???
}

pub fn allocate(addr: *mut (), page_count: uint)
{
	use arch::memory::addresses::is_global;
	unsafe
	{
		let pagenum = addr as uint / ::PAGE_SIZE;
		// 1. Lock
		let mut l = tern!( is_global(addr as uint) ? s_kernelspace_lock.lock() : s_userspace_lock.lock() );
		// 2. Ensure range is free
		for pg in range(pagenum, pagenum+page_count)
		{
			if ::arch::memory::virt::is_reserved(pg * ::PAGE_SIZE) {
				// nope.avi
			}
		}
		// 3. do `page_count` single arbitary allocations
		for pg in range(pagenum, pagenum+page_count)
		{
			::memory::phys::allocate( (pg * ::PAGE_SIZE) as *mut () );
		}
	}
	fail!("TODO: ::memory::virt::allocate(addr={}, page_count={})", addr, page_count);
}

pub fn map(addr: *mut (), phys: PAddr, prot: ProtectionMode)
{
	if ! ::arch::memory::virt::is_reserved(addr as VAddr)
	{
		::arch::memory::virt::map(addr, phys, prot);
	}
}

// vim: ft=rust
