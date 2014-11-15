// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/virt.rs
// - Virtual memory manager
use _common::*;
use core::ptr::RawPtr;
use arch::memory::addresses;

use arch::memory::{PAddr,VAddr};

type Page = [u8, ..::PAGE_SIZE];

#[deriving(PartialEq,Show)]
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

pub struct AllocHandle
{
	addr: *const (),
	count: uint,
}

#[link_section=".process_local"]
#[allow(non_upper_case_globals)]
static s_userspace_lock : ::sync::Mutex<()> = mutex_init!( () );
#[allow(non_upper_case_globals)]
static s_kernelspace_lock : ::sync::Mutex<()> = mutex_init!( () );

pub fn init()
{
	// 1. Tell the architecture-specific VMM that it can clean up init state
	// 2. ???
}

/// Ensure that the provded pages are valid (i.e. backed by memory)
pub fn allocate(addr: *mut (), page_count: uint)
{
	use arch::memory::addresses::is_global;
	unsafe
	{
		let pagenum = addr as uint / ::PAGE_SIZE;
		// 1. Lock
		let _lh = tern!( is_global(addr as uint) ? s_kernelspace_lock.lock() : s_userspace_lock.lock() );
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
}

pub fn map(addr: *mut (), phys: PAddr, prot: ProtectionMode)
{
	log_trace!("map(*{} := {:#x} {})", addr, phys, prot);
	if ::arch::memory::virt::is_reserved(addr as VAddr)
	{
		log_notice!("Mapping {:#x} to {}, collision", phys, addr);
	}
	else
	{
		::arch::memory::virt::map(addr, phys, prot);
	}
}

fn unmap(addr: *mut (), count: uint)
{
	log_trace!("unmap(*{} {})", addr, count);
	let _lock = s_kernelspace_lock.lock();
	let pos = addr as uint;
	
	{
		let ofs = pos & (::PAGE_SIZE - 1);
		if ofs != 0 {
			panic!("Non-aligned page {} passed (unmapping {} pages)", addr, count);
		}
	}
	
	for i in range(0, count)
	{
		::arch::memory::virt::unmap( (pos + i*::PAGE_SIZE) as *mut () );
	}
}

/*
/// Map a physical page for a short period of time (typically long enough to copy data in/out)
pub fn map_short(phys: PAddr) -> AllocHandle
{
	
}
*/

/// Create a long-standing MMIO/other hardware mapping
pub fn map_hw_ro(phys: PAddr, count: uint, module: &'static str) -> Result<AllocHandle,()> {
	map_hw(phys, count, true, module)
}
pub fn map_hw_rw(phys: PAddr, count: uint, module: &'static str) -> Result<AllocHandle,()> {
	map_hw(phys, count, false, module)
}

fn map_hw(phys: PAddr, count: uint, readonly: bool, _module: &'static str) -> Result<AllocHandle,()>
{
	// 1. Locate an area
	// TODO: This lock should be replaced with a finer grained lock
	let _lock = s_kernelspace_lock.lock();
	let mut pos = addresses::HARDWARE_BASE;
	loop
	{
		if addresses::HARDWARE_END - pos < count * ::PAGE_SIZE 
		{
			return Err( () );
		}
		let free = count_free_in_range(pos as *const Page, count);
		if free == count {
			break
		}
		pos += (free + 1) * ::PAGE_SIZE;
	}
	// 2. Map
	for i in range(0, count)
	{
		map(
			(pos + i * ::PAGE_SIZE) as *mut (),
			phys + (i * ::PAGE_SIZE) as u64,
			tern!(readonly ? ProtKernelRO : ProtKernelRW)
			);
	}
	// 3. Return a handle representing this area
	Ok( AllocHandle {
		addr: pos as *const _,
		count: count
		} )
}

fn count_free_in_range(addr: *const Page, count: uint) -> uint
{
	for i in range(0, count)
	{
		let pg = unsafe{ addr.offset(i as int) as uint };
		if ::arch::memory::virt::is_reserved( pg ) {
			return i;
		}
	}
	return count;
}

impl AllocHandle
{
	pub fn as_ref<'s,T>(&'s self, ofs: uint) -> &'s mut T
	{
		assert!(ofs + ::core::mem::size_of::<T>() <= self.count * ::PAGE_SIZE);
		unsafe{ &mut *((self.addr as uint + ofs) as *mut T) }
	}
	/// Forget the allocation and return a static reference to the data
	pub fn make_static<T>(&mut self, ofs: uint) -> &'static mut T
	{
		assert!(ofs + ::core::mem::size_of::<T>() <= self.count * ::PAGE_SIZE);
		self.count = 0;
		unsafe{ &mut *((self.addr as uint + ofs) as *mut T) }
	}
}
impl Drop for AllocHandle
{
	fn drop(&mut self)
	{
		if self.count > 0
		{
			unmap(self.addr as *mut (), self.count);
		}
	}
}

// vim: ft=rust
