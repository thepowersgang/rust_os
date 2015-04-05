// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/virt.rs
// - Virtual memory manager
use _common::*;
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
unsafe impl Send for AllocHandle {}

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

fn unmap(addr: *mut (), count: usize)
{
	log_trace!("unmap(*{:p} {})", addr, count);
	let _lock = s_kernelspace_lock.lock();
	let pos = addr as usize;
	
	let ofs = pos & (::PAGE_SIZE - 1);
	if ofs != 0 {
		panic!("Non-aligned page {:p} passed (unmapping {} pages)", addr, count);
	}
	
	for i in (0 .. count)
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
	if let Some(v) = ::arch::memory::virt::fixed_alloc(phys, count)
	{
		log_debug!("phys = {:#x}, v = {:#x}", phys, v);
		return Ok( AllocHandle {
			addr: v as *const _,
			count: count,
			mode: ProtectionMode::Unmapped,
			} );
	}

	let mode = if readonly { ProtectionMode::KernelRO } else { ProtectionMode::KernelRW };
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
	// 3. Return a handle representing this area
	Ok( AllocHandle {
		addr: pos as *const _,
		count: count,
		mode: mode,
		} )
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

impl AllocHandle
{
	pub fn as_ref<'s,T>(&'s self, ofs: usize) -> &'s mut T
	{
		&mut self.as_mut_slice(ofs, 1)[0]
	}
	/// Forget the allocation and return a static reference to the data
	pub fn make_static<T>(mut self, ofs: usize) -> &'static mut T
	{
		assert!(ofs % ::core::mem::align_of::<T>() == 0);
		assert!(ofs + ::core::mem::size_of::<T>() <= self.count * ::PAGE_SIZE);
		self.count = 0;
		unsafe{ &mut *((self.addr as usize + ofs) as *mut T) }
	}

	pub fn as_slice<T>(&self, ofs: usize, count: usize) -> &[T]
	{
		assert!( ofs % ::core::mem::align_of::<T>() == 0, "Offset {:#x} not aligned to {} bytes", ofs, ::core::mem::align_of::<T>());	// Align check
		assert!( ofs <= self.count * ::PAGE_SIZE );
		assert!( count * ::core::mem::size_of::<T>() <= self.count * ::PAGE_SIZE );
		assert!( ofs + count * ::core::mem::size_of::<T>() <= self.count * ::PAGE_SIZE );
		unsafe {
			::core::mem::transmute( ::core::raw::Slice {
				data: (self.addr as usize + ofs) as *const T,
				len: count,
			} )
		}
	}
	pub fn as_mut_slice<T>(&self, ofs: usize, count: usize) -> &mut [T]
	{
		assert!( self.mode == ProtectionMode::KernelRW );
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
}
impl ::core::fmt::Debug for AllocHandle
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		write!(f, "{:p}+{}pg", self.addr, self.count)
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
		self.alloc.as_ref( index * ::core::mem::size_of::<T>() )
	}
}

// vim: ft=rust
