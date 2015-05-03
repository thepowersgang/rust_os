// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/virt.rs
// - Virtual memory manager
use prelude::*;
use core::fmt;
use arch::memory::addresses;
use arch::memory::PAddr;

type Page = [u8; ::PAGE_SIZE];

#[derive(PartialEq,Debug,Copy,Clone)]
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

#[derive(Copy,Clone,Debug)]
pub enum MapError
{
	OutOfMemory,
	RangeInUse,
}

/// A handle to an owned memory allocation
pub struct AllocHandle
{
	addr: *const (),
	count: usize,
	mode: ProtectionMode,
}
unsafe impl Send for AllocHandle {}	// AllocHandle is sendable
unsafe impl Sync for AllocHandle {}	// &AllocHandle is safe
///// A single page from an AllocHandle
//pub struct PageHandle<'a>
//{
//	alloc: &'a mut AllocHandle,
//	idx: usize,
//}

/// A wrapper around AllocHandle that acts like an array
pub struct ArrayHandle<T>
{
	alloc: AllocHandle,
	_ty: ::core::marker::PhantomData<T>,
}

/// A wrapper around AllocHandle that acts like an array
pub struct SliceAllocHandle<T>
{
	alloc: AllocHandle,
	ofs: usize,
	count: usize,
	_ty: ::core::marker::PhantomData<T>,
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

// Alias the arch's get_phys method into this namespace
pub use arch::memory::virt::get_phys;

/// Ensure that the provded pages are valid (i.e. backed by memory)
pub fn allocate(addr: *mut (), page_count: usize)
{
	use arch::memory::addresses::is_global;

	let pagenum = addr as usize / ::PAGE_SIZE;
	// 1. Lock
	let _lh = if is_global(addr as usize) { s_kernelspace_lock.lock() } else { s_userspace_lock.lock() };
	// 2. Ensure range is free
	for pg in pagenum .. pagenum+page_count
	{
		let pgptr = (pg * ::PAGE_SIZE) as *const ();
		if ::arch::memory::virt::is_reserved( pgptr ) {
			// nope.avi
			panic!("TODO: Allocated memory ({:p}) in allocate({:p},{})", pgptr, addr, page_count);
		}
	}
	// 3. do `page_count` single arbitary allocations
	for pg in pagenum .. pagenum+page_count
	{
		::memory::phys::allocate( (pg * ::PAGE_SIZE) as *mut () );
	}
}

pub fn map(addr: *mut (), phys: PAddr, prot: ProtectionMode)
{
	//log_trace!("map(*{:p} := {:#x} {:?})", addr, phys, prot);
	if ::arch::memory::virt::is_reserved(addr)
	{
		log_notice!("Mapping {:#x} to {:p}, collision", phys, addr);
	}
	else
	{
		::arch::memory::virt::map(addr, phys, prot);
	}
}

pub unsafe fn unmap(addr: *mut (), count: usize)
{
	if ::arch::memory::virt::is_fixed_alloc(addr, count)
	{
		// Do nothing
		log_trace!("unmap(*{:p} {}) - Fixed alloc", addr, count);
	}
	else
	{	
		log_trace!("unmap(*{:p} {}) - Dynamic alloc", addr, count);
		let _lock = s_kernelspace_lock.lock();
		let pos = addr as usize;
		
		let ofs = pos & (::PAGE_SIZE - 1);
		if ofs != 0 {
			panic!("Non-aligned page {:p} passed (unmapping {} pages)", addr, count);
		}
		
		for i in (0 .. count) {
			::arch::memory::virt::unmap( (pos + i*::PAGE_SIZE) as *mut () );
		}
	}
}

/*
/// Map a physical page for a short period of time (typically long enough to copy data in/out)
pub fn map_short(phys: PAddr) -> AllocHandle
{
	
}
*/

/// Create a long-standing MMIO/other hardware mapping
pub fn map_hw_ro(phys: PAddr, count: usize, module: &'static str) -> Result<AllocHandle,MapError> {
	map_hw(phys, count, true, module)
}
pub fn map_hw_rw(phys: PAddr, count: usize, module: &'static str) -> Result<AllocHandle,MapError> {
	map_hw(phys, count, false, module)
}

/// Return a slice from physical memory
pub fn map_hw_slice<T>(phys: PAddr, num: usize) -> Result<SliceAllocHandle<T>,MapError>
{
	let ofs = phys & (::PAGE_SIZE - 1) as PAddr;
	let pa = phys - ofs;
	let count = ( (num * ::core::mem::size_of::<T>() + ofs as usize) + (::PAGE_SIZE - 1) ) / ::PAGE_SIZE;
	log_debug!("phys = {:#x}, {:#x}+{:#x}, count = {}", phys, pa, ofs, count);
	Ok (SliceAllocHandle {
		alloc: try!(map_hw_ro(pa, count, "")),
		ofs: ofs as usize,
		count: num,
		_ty: ::core::marker::PhantomData::<T>,
		} )	
}

fn map_hw(phys: PAddr, count: usize, readonly: bool, _module: &'static str) -> Result<AllocHandle,MapError>
{
	let mode = if readonly { ProtectionMode::KernelRO } else { ProtectionMode::KernelRW };
	
	if let Some(v) = ::arch::memory::virt::fixed_alloc(phys, count)
	{
		log_trace!("map_hw: Fixed allocation {:#x} => {:p}", phys, v as *const ());
		return Ok( AllocHandle {
			addr: v as *const _,
			count: count,
			mode: mode,
			} );
	}
	else
	{
		// 1. Locate an area
		// TODO: This lock should be replaced with a finer grained lock
		let _lock = s_kernelspace_lock.lock();
		let mut pos = addresses::HARDWARE_BASE;
		loop
		{
			if addresses::HARDWARE_END - pos < count * ::PAGE_SIZE 
			{
				return Err( MapError::RangeInUse );
			}
			let free = count_free_in_range(pos as *const Page, count);
			if free == count {
				break
			}
			pos += (free + 1) * ::PAGE_SIZE;
		}
		// 2. Map
		for i in (0 .. count)
		{
			map(
				(pos + i * ::PAGE_SIZE) as *mut (),
				phys + (i * ::PAGE_SIZE) as u64,
				mode
				);
		}
		log_trace!("map_hw: Dynamic allocation {:#x} => {:p}", phys, pos as *const ());
		// 3. Return a handle representing this area
		Ok( AllocHandle {
			addr: pos as *const _,
			count: count,
			mode: mode,
			} )
	}
}

pub fn alloc_dma(bits: u8, count: usize, module: &'static str) -> Result<AllocHandle,MapError>
{
	// 1. Allocate enough pages within the specified range
	let phys = ::memory::phys::allocate_range_bits(bits, count);
	if phys == ::memory::phys::NOPAGE {
		return Err( MapError::OutOfMemory );
	}
	// 2. Map that
	map_hw(phys, count, false, module)
}

fn count_free_in_range(addr: *const Page, count: usize) -> usize
{
	for i in (0 .. count)
	{
		let pg = unsafe { addr.offset(i as isize) };
		if ::arch::memory::virt::is_reserved( pg ) {
			return i;
		}
	}
	return count;
}

pub fn alloc_stack() -> AllocHandle
{
	let _lock = s_kernelspace_lock.lock();
	let mut pos = addresses::STACKS_BASE;
	while pos < addresses::STACKS_END
	{
		if ! ::arch::memory::virt::is_reserved( (pos + addresses::STACK_SIZE - ::PAGE_SIZE) as *const () )
		{
			let count = addresses::STACK_SIZE / ::PAGE_SIZE;
			for ofs in (0 .. count).map(|x| x * ::PAGE_SIZE)
			{
				::memory::phys::allocate( (pos + ofs) as *mut () );
			}
			// 3. Return a handle representing this area
			return AllocHandle {
				addr: pos as *const _,
				count: count,
				mode: ProtectionMode::KernelRW,
				};
		}
		pos += addresses::STACK_SIZE;
	}
	panic!("ERROR: Out of stacks");
}

impl fmt::Display for MapError
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		match *self
		{
		MapError::RangeInUse => write!(f, "Requested range is in use"),
		MapError::OutOfMemory => write!(f, "Out of memory"),
		}
	}
}

//pub struct PagesIterator<'a> {
//	alloc: &'a mut AllocHandle,
//	idx: usize,
//}
impl AllocHandle
{
	pub fn count(&self) -> usize {
		self.count
	}
	
	/// Borrow as T
	pub fn as_ref<T>(&self, ofs: usize) -> &T
	{
		&self.as_slice(ofs, 1)[0]
	}
	/// Mutably borrow as a T
	pub fn as_mut<T>(&mut self, ofs: usize) -> &mut T
	{
		&mut self.as_mut_slice(ofs, 1)[0]
	}
	/// Return a mutable borrow of the content (interior mutable)
	pub unsafe fn as_int_mut<T>(&self, ofs: usize) -> &mut T
	{
		&mut self.as_int_mut_slice(ofs, 1)[0]
	}
	/// Forget the allocation and return a static reference to the data
	pub fn make_static<T>(mut self, ofs: usize) -> &'static mut T
	{
		assert!(super::buf_valid(self.addr, self.count*0x1000));
		assert!(ofs % ::core::mem::align_of::<T>() == 0);
		assert!(ofs + ::core::mem::size_of::<T>() <= self.count * ::PAGE_SIZE);
		self.count = 0;
		unsafe{ &mut *((self.addr as usize + ofs) as *mut T) }
	}

	pub fn as_slice<T>(&self, ofs: usize, count: usize) -> &[T]
	{
		use core::mem::{align_of,size_of};
		assert!(super::buf_valid(self.addr, self.count*0x1000));
		assert!( ofs % align_of::<T>() == 0,
			"Offset {:#x} not aligned to {} bytes (T={})", ofs, align_of::<T>(), type_name!(T));
		assert!( ofs <= self.count * ::PAGE_SIZE,
			"Slice offset {} outside alloc of {} bytes", ofs, self.count*::PAGE_SIZE );
		assert!( count * size_of::<T>() <= self.count * ::PAGE_SIZE,
			"Entry count exceeds allocation ({} > {})", count * size_of::<T>(), self.count*::PAGE_SIZE);
		assert!( ofs + count * size_of::<T>() <= self.count * ::PAGE_SIZE,
			"Sliced region exceeds bounds {}+{} > {}", ofs, count * size_of::<T>(), self.count*::PAGE_SIZE);
		unsafe {
			::core::mem::transmute( ::core::raw::Slice {
				data: (self.addr as usize + ofs) as *const T,
				len: count,
			} )
		}
	}
	pub unsafe fn as_int_mut_slice<T>(&self, ofs: usize, count: usize) -> &mut [T]
	{
		assert!( self.mode == ProtectionMode::KernelRW,
			"Calling as_int_mut_slice<{}> on non-writable memory ({:?})", type_name!(T), self.mode );
		// Very evil, transmute the immutable slice into a mutable
		::core::mem::transmute( self.as_slice::<T>(ofs, count) )
	}
	pub fn as_mut_slice<T>(&mut self, ofs: usize, count: usize) -> &mut [T]
	{
		assert!( self.mode == ProtectionMode::KernelRW,
			"Calling as_mut_slice<{}> on non-writable memory ({:?})", type_name!(T), self.mode );
		unsafe {
			// Very evil, transmute the immutable slice into a mutable
			::core::mem::transmute( self.as_slice::<T>(ofs, count) )
		}
	}
	pub fn into_array<T>(self) -> ArrayHandle<T>
	{
		ArrayHandle {
			alloc: self,
			_ty: ::core::marker::PhantomData,
		}
	}
	
	//pub fn pages(&mut self) -> PagesIterator {
	//	PagesIterator {
	//		alloc: self,
	//		idx: 0,
	//	}
	//}
}
impl ::core::fmt::Debug for AllocHandle
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		write!(f, "{:p}+{}pg ({:?})", self.addr, self.count, self.mode)
	}
}
impl Drop for AllocHandle
{
	fn drop(&mut self)
	{
		if self.count > 0
		{
			// SAFE: Dropping an allocation controlled by this object
			unsafe { unmap(self.addr as *mut (), self.count); }
			self.count = 0;
		}
	}
}

//impl<'a> ::core::iter::Iterator for PagesIterator<'a>
//{
//	type Item = PageHandle<'a>;
//	fn next(&mut self) -> Option<PageHandle<'a>> {
//		if self.idx < self.alloc.count {
//			self.idx += 1;
//			Some(PageHandle {
//				// Erase the lifetime
//				// SAFE: PageHandle doesn't expose the alloc handle (and we don't give out duplicates)
//				alloc: &mut unsafe { *(self.alloc as *mut _) },
//				idx: self.idx - 1,
//			})
//		}
//		else {
//			None
//		}
//	}
//}
//impl<'a> PageHandle<'a>
//{
//	pub unsafe fn map(&mut self, paddr: PAddr) -> Result<(),()> {
//		unimplemented!();
//	}
//	pub unsafe fn map_cow(&mut self, paddr: PAddr) -> Result<(),()> {
//		unimplemented!();
//	}
//}
//impl<'a> ::core::convert::AsRef<[u8]> for PageHandle<'a>
//{
//	fn as_ref(&self) -> &[u8] { self.alloc.as_slice(self.idx * 0x1000, 0x1000) }
//}
//impl<'a> ::core::convert::AsMut<[u8]> for PageHandle<'a>
//{
//	fn as_mut(&mut self) -> &mut [u8] { self.alloc.as_mut_slice(self.idx * 0x1000, 0x1000) }
//}

impl<T> SliceAllocHandle<T>
{
}

impl<T> ::core::ops::Deref for SliceAllocHandle<T>
{
	type Target = [T];
	fn deref(&self) -> &[T]
	{
		self.alloc.as_slice(self.ofs, self.count)
	}
}

impl<T> ArrayHandle<T>
{
	pub fn len(&self) -> usize {
		self.alloc.count * ::PAGE_SIZE / ::core::mem::size_of::<T>()
	}
}

impl<T> ::core::ops::Index<usize> for ArrayHandle<T>
{
	type Output = T;
	fn index<'a>(&'a self, index: usize) -> &'a T {
		self.alloc.as_ref( index * ::core::mem::size_of::<T>() )
	}
}

impl<T> ::core::ops::IndexMut<usize> for ArrayHandle<T>
{
	fn index_mut<'a>(&'a mut self, index: usize) -> &'a mut T {
		self.alloc.as_mut( index * ::core::mem::size_of::<T>() )
	}
}

// vim: ft=rust
