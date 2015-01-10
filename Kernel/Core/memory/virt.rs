// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/virt.rs
// - Virtual memory manager
use _common::*;
use arch::memory::addresses;

use arch::memory::{PAddr,VAddr};

type Page = [u8; ::PAGE_SIZE];

#[derive(PartialEq,Show,Copy)]
pub enum ProtectionMode
{	
	Unmapped,	// Inaccessible
	KernelRO,	// Kernel readonly
	KernelRW,	// Kernel read-write
	KernelRX,	// Kernel read-execute
	UserRO,	// User
	UserRW,
	UserRX,
}

pub struct AllocHandle
{
	addr: *const (),
	count: uint,
	mode: ProtectionMode,
}
unsafe impl Send for AllocHandle {}

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

	let pagenum = addr as uint / ::PAGE_SIZE;
	// 1. Lock
	let _lh = if is_global(addr as uint) { s_kernelspace_lock.lock() } else { s_userspace_lock.lock() };
	// 2. Ensure range is free
	for pg in range(pagenum, pagenum+page_count)
	{
		let pgptr = (pg * ::PAGE_SIZE) as *const ();
		if ::arch::memory::virt::is_reserved( pgptr ) {
			// nope.avi
			panic!("TODO: Already reserved memory in range passed to allocate({:p},{}) ({:p})", addr, page_count, pgptr);
		}
	}
	// 3. do `page_count` single arbitary allocations
	for pg in range(pagenum, pagenum+page_count)
	{
		::memory::phys::allocate( (pg * ::PAGE_SIZE) as *mut () );
	}
}

pub fn map(addr: *mut (), phys: PAddr, prot: ProtectionMode)
{
	log_trace!("map(*{:p} := {:#x} {:?})", addr, phys, prot);
	if ::arch::memory::virt::is_reserved(addr)
	{
		log_notice!("Mapping {:#x} to {:p}, collision", phys, addr);
	}
	else
	{
		::arch::memory::virt::map(addr, phys, prot);
	}
}

fn unmap(addr: *mut (), count: uint)
{
	log_trace!("unmap(*{:p} {})", addr, count);
	let _lock = s_kernelspace_lock.lock();
	let pos = addr as uint;
	
	let ofs = pos & (::PAGE_SIZE - 1);
	if ofs != 0 {
		panic!("Non-aligned page {:p} passed (unmapping {} pages)", addr, count);
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
	let mode = if readonly { ProtectionMode::KernelRO } else { ProtectionMode::KernelRW };
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
			mode
			);
	}
	// 3. Return a handle representing this area
	Ok( AllocHandle {
		addr: pos as *const _,
		count: count,
		mode: mode,
		} )
}

fn count_free_in_range(addr: *const Page, count: uint) -> uint
{
	for i in range(0, count)
	{
		let pg = unsafe { addr.offset(i as int) };
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

	pub fn as_slice<T>(&self, ofs: uint, count: uint) -> &[T]
	{
		assert!( ofs % ::core::mem::align_of::<T>() == 0 );	// Align check
		assert!( ofs <= self.count * ::PAGE_SIZE );
		assert!( count * ::core::mem::size_of::<T>() <= self.count * ::PAGE_SIZE );
		assert!( ofs + count * ::core::mem::size_of::<T>() <= self.count * ::PAGE_SIZE );
		unsafe {
			::core::mem::transmute( ::core::raw::Slice {
				data: (self.addr as uint + ofs) as *const T,
				len: count,
			} )
		}
	}
	pub fn as_mut_slice<T>(&mut self, ofs: uint, count: uint) -> &mut [T]
	{
		assert!( self.mode == ProtectionMode::KernelRW );
		unsafe {
			// Very evil, transmute the immutable slice into a mutable
			::core::mem::transmute( self.as_slice::<T>(ofs, count) )
		}
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
