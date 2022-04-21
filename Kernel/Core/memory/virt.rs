// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/virt.rs
//! Virtual memory management - DMA and temprory mappings
use core::fmt;
use core::ops;
use crate::arch::memory::addresses;
use crate::arch::memory::{PAddr, PAGE_MASK};
use crate::PAGE_SIZE;

type Page = [u8; PAGE_SIZE];

#[derive(PartialEq,Debug,Copy,Clone)]
/// Memory protection flags
pub enum ProtectionMode
{
	/// Inaccessible
	Unmapped,
	/// Kernel readonly
	KernelRO,
	KernelRW,	// Kernel read-write
	KernelRX,	// Kernel read-execute
	UserRO,	// User
	UserRW,
	UserRX,
	UserCOW,	// User Copy-on-write (becomes UserRW on write)
	UserRWX,	// AVOID - Read-Write-Execute (exists for internal reasons)
}

#[derive(Copy,Clone,Debug)]
pub enum MapError
{
	OutOfMemory,
	RangeInUse,
}
impl_from! {
	From<crate::memory::phys::Error>(_v) for MapError {
		MapError::OutOfMemory
	}
	From<MapError>(v) for &'static str {
		match v {
		MapError::OutOfMemory => "VMM: Out of memory",
		MapError::RangeInUse => "VMM: Range in use",
		}
	}
}

/// A handle to an arbitary owned memory allocation.
pub struct AllocHandle
{
	value: ::core::num::NonZeroUsize,
	//addr: *const (),
	//count: usize,
	//mode: ProtectionMode,
}
unsafe impl Send for AllocHandle {}	// AllocHandle is sendable
unsafe impl Sync for AllocHandle {}	// &AllocHandle is safe

/// A wrapper around AllocHandle that acts like an array
pub struct ArrayHandle<T: crate::lib::POD>
{
	alloc: AllocHandle,
	_ty: ::core::marker::PhantomData<T>,
}

/// A wrapper around AllocHandle that acts like an array
pub struct SliceAllocHandle<T: crate::lib::POD>
{
	alloc: AllocHandle,
	ofs: usize,
	count: usize,
	_ty: ::core::marker::PhantomData<T>,
}

#[link_section=".process_local"]
#[allow(non_upper_case_globals)]
static s_userspace_lock : crate::sync::Mutex<()> = mutex_init!( () );
#[allow(non_upper_case_globals)]
static s_kernelspace_lock : crate::sync::Mutex<()> = mutex_init!( () );

#[doc(hidden)]
pub fn init()
{
	// 1. Tell the architecture-specific VMM that it can clean up init state
	crate::arch::memory::virt::post_init();
	// 2. ???
}

struct Pages(*mut (), usize);
impl ::core::iter::Iterator for Pages {
	type Item = *mut ();
	fn next(&mut self) -> Option<*mut ()> {
		if self.1 == 0 {
			None
		}
		else {
			let rv = self.0;
			self.0 = (rv as usize + PAGE_SIZE) as *mut ();
			self.1 -= 1;
			Some(rv)
		}
	}
}

// Alias the arch's get_phys method into this namespace
pub use crate::arch::memory::virt::is_reserved;
pub use crate::arch::memory::virt::get_phys;
pub use crate::arch::memory::virt::get_info;

/// Temporarily map a frame into memory and run the provided closure
pub unsafe fn with_temp<F, R>(phys: PAddr, f: F) -> R
where
	F: FnOnce(&mut [u8; PAGE_SIZE]) -> R
{
	assert!(phys & PAGE_MASK as PAddr == 0, "Unaligned address passed to with_temp");
	let mut th = crate::arch::memory::virt::TempHandle::<u8>::new(phys);
	let p: &mut [u8; PAGE_SIZE] = ::core::mem::transmute(&mut th[0]);
	f(p)
}

/// Run the provided closure with no changes possible to the address space
pub fn with_lock<F>(addr: usize, fcn: F)
where
	F: FnOnce()
{
	let _lh = if crate::arch::memory::addresses::is_global(addr) { s_kernelspace_lock.lock() } else { s_userspace_lock.lock() };
	fcn();
}

/// Ensure that the provded pages are valid (i.e. backed by memory)
pub fn allocate(addr: *mut (), page_count: usize) -> Result<(), MapError> {
	allocate_int(addr, page_count, false)
}
/// Allocate memory for user access
pub fn allocate_user(addr: *mut (), page_count: usize) -> Result<(), MapError> {
	allocate_int(addr, page_count, true)
}

fn allocate_int(addr: *mut (), page_count: usize, is_user: bool) -> Result<(), MapError>
{
	use crate::arch::memory::addresses::is_global;

	// 1. Lock
	let _lh = if is_global(addr as usize) { s_kernelspace_lock.lock() } else { s_userspace_lock.lock() };
	// 2. Ensure range is free
	for pgptr in Pages(addr, page_count)
	{
		if crate::arch::memory::virt::is_reserved( pgptr ) {
			// nope.avi
			log_warning!("Allocated memory ({:p}) in allocate({:p},{})", pgptr, addr, page_count);
			return Err(MapError::RangeInUse);
		}
	}
	// 3. do `page_count` single arbitary allocations
	for pgptr in Pages(addr, page_count) {
		if ! crate::memory::phys::allocate( pgptr ) {
			// Allocation error!
			let n_done = (pgptr as usize - addr as usize) / PAGE_SIZE;
			for pgptr in Pages(addr, n_done) {
				// SAFE: We've just made these valid, thus we own them
				unsafe {
					if let Some(pa) = crate::arch::memory::virt::unmap(pgptr) {
						crate::memory::phys::deref_frame(pa);
					}
				}
			}

			return Err( MapError::OutOfMemory );
		}
	}
	if is_user {
		for pgptr in Pages(addr, page_count) {
			// SAFE: This region has just been allocated, and is KernelRW, upgrading to allow user access
			unsafe {
				crate::arch::memory::virt::reprotect(pgptr, ProtectionMode::UserRW);
			}
		}
	}

	Ok( () )
}

/// Atomically reserves a region of address space
pub fn reserve(addr: *mut (), page_count: usize) -> Result<Reservation, ()>
{
	use crate::arch::memory::addresses::is_global;
	
	if is_global(addr as usize) != is_global(addr as usize + page_count * PAGE_SIZE - 1) {
		todo!("Error out when straddling user-supervisor region {:p}+{:#x}", addr, page_count*PAGE_SIZE)
	}
	
	assert_eq!(addr as usize % PAGE_SIZE, 0);
	
	// 1. Lock
	let _lh = if is_global(addr as usize) { s_kernelspace_lock.lock() } else { s_userspace_lock.lock() };
	// 2. Ensure range is free
	for pgptr in Pages(addr, page_count)
	{
		if crate::arch::memory::virt::is_reserved( pgptr ) {
			log_trace!("Address {:?} in range {:p}+{}pg reserved", pgptr, addr, page_count);
			return Err( () );
		}
	}
	// 3. do `page_count` single arbitary allocations
	for pgptr in Pages(addr, page_count)
	{
		// TODO: Instead map in COW zero pages
		crate::memory::phys::allocate( pgptr );
	}
	
	Ok( Reservation(addr, page_count) )
}
/// Handle to a reserved region of address space
pub struct Reservation(*mut (), usize);
impl Reservation
{
	pub fn get_mut_page(&mut self, idx: usize) -> &mut [u8] {
		assert!(idx < self.1);
		// SAFE: Unique, and owned
		unsafe { ::core::slice::from_raw_parts_mut( (self.0 as usize + idx * PAGE_SIZE) as *mut u8, PAGE_SIZE) }
	}
	//pub fn map_at(&mut self, idx: usize, frame: FrameHandle) -> Result<(),FrameHandle> {
	//	assert!(idx < self.1);
	//	let addr = (self.0 as usize + idx * PAGE_SIZE) as *mut ();
	//	::arch::memory::virt::unmap(addr);
	//	::arch::memory::virt::map(addr, frame.into_addr(), ProtectionMode::KernelRW);
	//	Ok( () )
	//}
	pub fn finalise(self, final_mode: ProtectionMode) -> Result<(),()> {
		log_trace!("Reservation::finalise(final_mode={:?})", final_mode);
		for addr in Pages(self.0, self.1) {
			// SAFE: Just changing flags, and 'self' owns this region of memory.
			unsafe {
				crate::arch::memory::virt::reprotect(addr, final_mode);
			}
		}
		Ok( () )
	}
}

/// Map the given physical address to the given virtual address
/// UNSAFE: Does no checks on validity of the physical address. When deallocated, the mapped address will be dereferenced
pub unsafe fn map(addr: *mut (), phys: PAddr, prot: ProtectionMode)
{
	//log_trace!("map(*{:p} := {:#x} {:?})", addr, phys, prot);
	if crate::arch::memory::virt::is_reserved(addr)
	{
		log_notice!("Mapping {:#x} to {:p}, collision", phys, addr);
		crate::arch::print_backtrace();
		// TODO: This needs to return an error!
	}
	else
	{
		// XXX: TODO: This can race, and do what?
		crate::arch::memory::virt::map(addr, phys, prot);
	}
}

/// Alter the protection flags on a mapping (only allows changing to a user-accessible mode)
/// UNSAFE: (Very) Can change the protection mode of a page to anything
pub unsafe fn reprotect_user(addr: *mut (), prot: ProtectionMode) -> Result<(),()>
{
	match prot
	{
	ProtectionMode::Unmapped => {},
	ProtectionMode::UserRX => {},
	ProtectionMode::UserRO => {},
	_ => panic!("Invalid protection mode passed to reprotect_user - {:?}", prot),
	}
	if crate::arch::memory::addresses::is_global(addr as usize) {
		Err( () )
	}
	else if ! crate::arch::memory::virt::is_reserved(addr) {
		Err( () )
	}
	else {
		if prot == ProtectionMode::Unmapped {
			if let Some(paddr) = crate::arch::memory::virt::unmap(addr) {
				crate::memory::phys::deref_frame(paddr);
			}
		}
		else {
			crate::arch::memory::virt::reprotect(addr, prot);
		}
		Ok( () )
	}
}

/// Unmap the frame at the given virtual address
/// UNSAFE: (Very) invalidates the given pointer
pub unsafe fn unmap(addr: *mut (), count: usize)
{
	if crate::arch::memory::virt::is_fixed_alloc(addr, count)
	{
		// Do nothing
		//log_trace!("unmap(*{:p} {}) - Fixed alloc", addr, count);
	}
	else
	{	
		//log_trace!("unmap(*{:p} {}) - Dynamic alloc", addr, count);
		let _lock = s_kernelspace_lock.lock();
		let pos = addr as usize;
		
		let ofs = pos & (PAGE_SIZE - 1);
		if ofs != 0 {
			panic!("Non-aligned page {:p} passed (unmapping {} pages)", addr, count);
		}
		
		// Dereference the frames returned
		for i in 0 .. count {
			if let Some(addr) = crate::arch::memory::virt::unmap( (pos + i*PAGE_SIZE) as *mut () ) {
				crate::memory::phys::deref_frame(addr);
			}
		}
	}
}

/// Return a pointer to the given physical address in the fixed allocation region
///
/// Usually only works for addresses under 4MB
pub unsafe fn map_static_raw(phys: PAddr, size: usize) -> Result<*const crate::Void, MapError> {
	let ofs = phys as usize % PAGE_SIZE;
	let pages = (ofs + size + PAGE_SIZE - 1) / PAGE_SIZE;
	if let Some(p) = crate::arch::memory::virt::fixed_alloc(phys & !(PAGE_SIZE as PAddr - 1), pages) {
		log_trace!("{:#x}+{}pg is {:p}", phys, pages, p);
		Ok( (p as usize + ofs) as *const crate::Void)
	}
	else {
		log_trace!("{:#x}+{}pg not in fixed region", phys, pages);
		Err(MapError::OutOfMemory)
		//todo!("map_static_raw(phys={:#x}, size={:#x})", phys, size);
	}
}
/// Wraps `map_static_raw` and returns a `&'static [T]`
pub unsafe fn map_static_slice<T: crate::lib::POD>(phys: PAddr, count: usize) -> Result<&'static [T], MapError> {
	map_static_raw(phys, count * ::core::mem::size_of::<T>())
		.map(|ptr| ::core::slice::from_raw_parts(ptr as *const T, count))
}
/// Wraps `map_static_raw` and returns a `&'static T`
pub unsafe fn map_static<T: crate::lib::POD>(phys: PAddr) -> Result<&'static T, MapError> {
	map_static_raw(phys, ::core::mem::size_of::<T>())
		.map(|ptr| &*(ptr as *const T))
}

/// Handle to a region of memory to be used for MMIO. See [map_mmio](function.map_mmio.html)
// NOTE: Designed to take up only 2 words on a 32-bit platform
// If the size (second u16) is zero, it's a page-aligned region
pub struct MmioHandle(::core::ptr::NonNull<crate::Void>,u16,u16);
unsafe impl Send for MmioHandle {}	// MmioHandle is sendable
unsafe impl Sync for MmioHandle {}	// &MmioHandle is safe
impl_fmt! {
	Debug(self,f) for MmioHandle {
		write!(f, "{:p}({:#x})+{:#x}", self.base(), get_phys(self.base()), self.2)
	}
}
/// Maps the given physical address for memory-mapped IO access. NOTE: This address does not need to be page aligned.
pub unsafe fn map_mmio(phys: PAddr, size: usize) -> Result<MmioHandle,MapError> {
	let (phys_page, phys_ofs) = (phys & !(PAGE_MASK as PAddr), phys & PAGE_MASK as PAddr);

	let is_aligned = phys_ofs == 0 && size % PAGE_SIZE == 0 && size > 0;
	if is_aligned {
		// If aligned, this can handle up to 8MiB (23 bits) through bit packing in AllocHandle
		assert!(size < AllocHandle::MAX_SIZE, "map_mmio size {:#x} too large (must be below {:#x})", size, AllocHandle::MAX_SIZE);
	}
	else {
		assert!(size < (1<<16), "map_mmio size {:#x} too large (non-aligned allocs must be smaller than {:#x})", size, (1<<16));
	}

	let npages = (size + phys_ofs as usize + PAGE_SIZE - 1) / PAGE_SIZE;
	assert!(npages < (1<<16));
	let ah = map_hw(phys_page, npages, false, "MMIO")?;

	let p = ::core::ptr::NonNull::new_unchecked(ah.addr() as *mut _);
	let rv = if is_aligned {
			MmioHandle( p, npages as u16, 0 )
		}
		else {
			MmioHandle( p, phys_ofs as u16, size as u16 )
		};
	::core::mem::forget(ah);
	Ok(rv)
}
impl MmioHandle
{
	pub fn base(&self) -> *mut crate::Void {
		(self.0.as_ptr() as usize + self.ofs()) as *mut crate::Void
	}
	fn ofs(&self) -> usize {
		if self.2 == 0 {
			0
		}
		else {
			self.1 as usize
		}
	}
	pub fn size(&self) -> usize {
		if self.2 == 0 {
			self.1 as usize * PAGE_SIZE
		}
		else {
			self.2 as usize
		}
	}
	fn as_raw_ptr_slice<T>(&self, ofs: usize, count: usize) -> *mut [T]
	{
		use core::mem::{align_of,size_of};
		debug_assert!(super::buf_valid(self.base() as *const (), self.size()));
		assert!( ofs % align_of::<T>() == 0,
			"Offset {:#x} not aligned to {} bytes (T={})", ofs, align_of::<T>(), type_name!(T));
		assert!( ofs <= self.size(),
			"Slice offset {} outside alloc of {} bytes", ofs, self.size() );
		assert!( count * size_of::<T>() <= self.size(),
			"Entry count exceeds allocation ({} > {})", count * size_of::<T>(), self.size());
		assert!( ofs + count * size_of::<T>() <= self.size(),
			"Sliced region exceeds bounds {}+{} > {}", ofs, count * size_of::<T>(), self.size());
		// SAFE: Doesn't ensure lack of aliasing, but the address is valid. Immediately casted to a raw pointer, so aliasing is OK
		unsafe {
			::core::slice::from_raw_parts_mut( (self.base() as usize + ofs) as *mut T, count )
		}
	}
	/// Interpret the backing memory as a slice
	pub unsafe fn as_int_mut_slice<T: crate::lib::POD>(&self, ofs: usize, count: usize) -> &mut [T]
	{
		&mut (*self.as_raw_ptr_slice(ofs, count))[..]
	}
	/// Return a mutable borrow of the content (interior mutable)
	pub unsafe fn as_int_mut<T: crate::lib::POD>(&self, ofs: usize) -> &mut T
	{
		&mut self.as_int_mut_slice(ofs, 1)[0]
	}
	/// Return a mutable pointer to the content
	pub fn as_mut_ptr<T: crate::lib::POD>(&self, ofs: usize) -> *mut T {
		self.as_raw_ptr_slice::<T>(ofs, 1) as *mut T
	}

	pub fn phys(&self) -> PAddr {
		get_phys(self.base())
	}
}
impl ops::Drop for MmioHandle
{
	fn drop(&mut self)
	{
		// SAFE: Owned allocaton
		unsafe {
			unmap(self.0.as_ptr() as *mut (), (self.size() + PAGE_SIZE - 1) / PAGE_SIZE);
		}
	}
}

// TODO: Update these two methods to ENSURE that the memory passed isn't allocatable RAM (or has been invalidated in the PMM)
/// Create a long-standing MMIO/other hardware mapping
pub unsafe fn map_hw_ro(phys: PAddr, count: usize, module: &'static str) -> Result<AllocHandle,MapError> {
	map_hw(phys, count, true, module)
}
/// Create a long-standing MMIO/other hardware mapping (writable)
pub unsafe fn map_hw_rw(phys: PAddr, count: usize, module: &'static str) -> Result<AllocHandle,MapError> {
	map_hw(phys, count, false, module)
}

/// Return a slice from physical memory
///
/// UNSAFE: Can cause aliasing (but does handle referencing the memory)
pub unsafe fn map_hw_slice<T: crate::lib::POD>(phys: PAddr, num: usize) -> Result<SliceAllocHandle<T>,MapError>
{
	let ofs = phys & (PAGE_SIZE - 1) as PAddr;
	let pa = phys - ofs;
	let count = ( (num * ::core::mem::size_of::<T>() + ofs as usize) + (PAGE_SIZE - 1) ) / PAGE_SIZE;
	log_debug!("phys = {:#x}, {:#x}+{:#x}, count = {}", phys, pa, ofs, count);
	
	// - Reference all pages in the region
	for i in 0 .. count {
		crate::memory::phys::ref_frame(pa + (i * PAGE_SIZE) as PAddr);
	}
	
	// Map memory (using the raw map_hw call)
	Ok( SliceAllocHandle {
		alloc: map_hw(pa, count, true, "map_hw_slice")?,
		ofs: ofs as usize,
		count: num,
		_ty: ::core::marker::PhantomData::<T>,
		} )	
}

/// UNSAFE: Can be used to introduce aliasing on `phys` (and does not protect against double-deref caused by incorrectly mapping RAM)
unsafe fn map_hw(phys: PAddr, count: usize, readonly: bool, _module: &'static str) -> Result<AllocHandle,MapError>
{
	let mode = if readonly { ProtectionMode::KernelRO } else { ProtectionMode::KernelRW };
	
	if let Some(v) = crate::arch::memory::virt::fixed_alloc(phys, count)
	{
		log_trace!("map_hw: Fixed allocation {:#x} => {:p}", phys, v as *const ());
		return Ok( AllocHandle::new(v as usize, count, mode) );
	}
	else
	{
		// 1. Locate an area
		// TODO: This lock should be replaced with a finer grained lock
		let _lock = s_kernelspace_lock.lock();
		let mut pos = addresses::HARDWARE_BASE;
		loop
		{
			if addresses::HARDWARE_END - pos < count * PAGE_SIZE 
			{
				return Err( MapError::RangeInUse );
			}
			let free = count_free_in_range(pos as *const Page, count);
			if free == count {
				break
			}
			pos += (free + 1) * PAGE_SIZE;
		}
		// 2. Map
		for i in 0 .. count
		{
			map(
				(pos + i * PAGE_SIZE) as *mut (),
				phys + (i * PAGE_SIZE) as PAddr,
				mode
				);
		}
		log_trace!("map_hw: Dynamic allocation {:#x} => {:p}", phys, pos as *const ());
		// 3. Return a handle representing this area
		Ok( AllocHandle::new(pos, count, mode) )
	}
}

// TODO: Have a specialised allocator just for the disk/file cache. Like the heap.

/// Allocate a new page mapped in a temporary region, ready for use with memory-mapped files
// TODO: What else would use this? Should I just have it be "alloc file page" and take the file ID/offset?
// - May also be used by new process code
pub fn alloc_free() -> Result<FreePage,MapError>
{
	log_trace!("alloc_free()");
	let map_handle = crate::memory::phys::allocate_bare().map_err(|_| MapError::OutOfMemory)?;
	log_trace!("- frame = {:#x}, map_handle = {:p}", get_phys(&map_handle[0]), &map_handle[0]);
	Ok( FreePage(map_handle) )
}

/// Handle returned by [alloc_free](fn.alloc_free.html). This type panics on drop.
pub struct FreePage( crate::arch::memory::virt::TempHandle<u8> );
impl FreePage
{
	fn phys(&self) -> PAddr {
		get_phys( &self.0[0] )
	}
	/// Unmap the memory and return a handle to the backing frame
	pub fn into_frame(self) -> crate::memory::phys::FrameHandle {
		let paddr = self.phys();
		// SAFE: Forgets after read (used because Self::drop panics)
		unsafe {
			let _ = ::core::ptr::read(&self.0);
			::core::mem::forget(self);
		}
		// SAFE: Valid physical address passed
		unsafe { crate::memory::phys::FrameHandle::from_addr_noref(paddr) }
	}
	/// Interpret the page as a mutable slice of `T`
	pub fn as_slice_mut<T: crate::lib::POD>(&mut self) -> &mut [T] {
		// SAFE: Lifetime and range is valid, data is POD
		unsafe {
			::core::slice::from_raw_parts_mut( &mut self[0] as *mut u8 as *mut _, PAGE_SIZE / ::core::mem::size_of::<T>() )
		}
	}
}
impl ops::Drop for FreePage {
	fn drop(&mut self) {
		panic!("FreePage shouldn't be dropped");
	}
}
impl ops::Deref for FreePage {
	type Target = [u8];
	fn deref(&self) -> &[u8] {
		&self.0
	}
}
impl ops::DerefMut for FreePage {
	fn deref_mut(&mut self) -> &mut [u8] {
		&mut self.0
	}
}

/// Allocate memory allowing for hardware DMA restrictions
pub fn alloc_dma(bits: u8, count: usize, module: &'static str) -> Result<AllocHandle,MapError>
{
	// 1. Allocate enough pages within the specified range
	let phys = crate::memory::phys::allocate_range_bits(bits, count);
	if phys == crate::memory::phys::NOPAGE {
		return Err( MapError::OutOfMemory );
	}
	// 2. Map that
	// SAFE: Physical address has just been allocated
	unsafe {
		map_hw(phys, count, false, module)
	}
}

fn count_free_in_range(addr: *const Page, count: usize) -> usize
{
	for i in 0 .. count
	{
		// SAFE: Offset should be valid... (TODO: Ensure, and do bounds checking)
		let pg = unsafe { addr.offset(i as isize) };
		if crate::arch::memory::virt::is_reserved( pg ) {
			return i;
		}
	}
	return count;
}

/// Allocate a new kernel stack
pub fn alloc_stack() -> AllocHandle
{
	let _lock = s_kernelspace_lock.lock();
	let mut pos = addresses::STACKS_BASE;
	while pos < addresses::STACKS_END
	{
		if ! crate::arch::memory::virt::is_reserved( (pos + addresses::STACK_SIZE - PAGE_SIZE) as *const () )
		{
			let count = addresses::STACK_SIZE / PAGE_SIZE;
			for ofs in (1 .. count).map(|x| x * PAGE_SIZE)
			{
				crate::memory::phys::allocate( (pos + ofs) as *mut () );
			}
			// 3. Return a handle representing this area
			return AllocHandle::new( pos + PAGE_SIZE, count-1, ProtectionMode::KernelRW );
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

/*
impl Default for AllocHandle {
	fn default() -> AllocHandle {
		AllocHandle {
			value: 0,
			}
	}
}*/
impl AllocHandle
{
	const MAX_SIZE: usize = PAGE_SIZE * (PAGE_SIZE / 2 - 1);
	pub fn new(addr: usize, count: usize, mode: ProtectionMode) -> AllocHandle {
		assert!(addr != 0, "Zero pointer for AllocHandle");
		assert!(addr & (PAGE_SIZE - 1) == 0, "Non-aligned value for AllocHandle : {:#x}", addr);
		assert!(count < PAGE_SIZE / 2, "Over-sized allocation in AllocHandle : {} >= {}", count, PAGE_SIZE/2);
		AllocHandle {
			value: ::core::num::NonZeroUsize::new(addr | (count << 1) | (mode == ProtectionMode::KernelRW) as usize).unwrap(),
			}
	}
	fn addr(&self) -> *const () {
		(self.value.get() & !(PAGE_SIZE - 1)) as *const ()
	}
	pub fn is_mutable(&self) -> bool {
		(self.value.get() & 1) != 0
	}
	pub fn count(&self) -> usize {
		(self.value.get() & (PAGE_SIZE - 1)) >> 1
	}
	pub fn len(&self) -> usize {
		self.count() * PAGE_SIZE
	}
	
	/// Borrow as T
	pub fn as_ref<T: crate::lib::POD>(&self, ofs: usize) -> &T
	{
		&self.as_slice(ofs, 1)[0]
	}
	/// Mutably borrow as a T
	pub fn as_mut<T: crate::lib::POD>(&mut self, ofs: usize) -> &mut T
	{
		&mut self.as_mut_slice(ofs, 1)[0]
	}
	/// Return a mutable borrow of the content (interior mutable)
	pub unsafe fn as_int_mut<T: crate::lib::POD>(&self, ofs: usize) -> &mut T
	{
		&mut self.as_int_mut_slice(ofs, 1)[0]
	}
	/// Forget the allocation and return a static reference to the data
	pub fn make_static<T: crate::lib::POD>(self, ofs: usize) -> &'static mut T
	{
		debug_assert!(super::buf_valid(self.addr(), self.len()));
		assert!(ofs % ::core::mem::align_of::<T>() == 0);
		assert!(ofs + ::core::mem::size_of::<T>() <= self.len());
		assert!(self.is_mutable());
		let rv_ptr = (self.addr() as usize + ofs) as *mut T;
		::core::mem::forget(self);
		// SAFE: owned and Plain-old-data (setting count above to 0 ensures no deallocation)
		unsafe{ &mut *rv_ptr }
	}

	fn as_raw_ptr_slice<T>(&self, ofs: usize, count: usize) -> *mut [T]
	{
		use core::mem::{align_of,size_of};
		debug_assert!(super::buf_valid(self.addr(), self.len()));
		assert!( ofs % align_of::<T>() == 0,
			"Offset {:#x} not aligned to {} bytes (T={})", ofs, align_of::<T>(), type_name!(T));
		assert!( ofs <= self.len(),
			"Slice offset {} outside alloc of {} bytes", ofs, self.len() );
		assert!( count * size_of::<T>() <= self.count() * PAGE_SIZE,
			"Entry count exceeds allocation ({} > {})", count * size_of::<T>(), self.len());
		assert!( ofs + count * size_of::<T>() <= self.len(),
			"Sliced region exceeds bounds {}+{}*{} {} > {}", ofs, count, size_of::<T>(), ofs+count*size_of::<T>(), self.len());
		// SAFE: Doesn't ensure lack of aliasing, but the address is valid. Immediately coerced to a raw pointer, so aliasing is OK
		unsafe {
			::core::slice::from_raw_parts_mut( (self.addr() as usize + ofs) as *mut T, count )
		}
	}
	pub fn as_slice<T: crate::lib::POD>(&self, ofs: usize, count: usize) -> &[T]
	{
		// SAFE: & and Plain-old-data
		unsafe {
			&(*self.as_raw_ptr_slice(ofs, count))[..]
		}
	}
	pub unsafe fn as_int_mut_slice<T: crate::lib::POD>(&self, ofs: usize, count: usize) -> &mut [T]
	{
		assert!(self.is_mutable(),
			"Calling as_int_mut_slice<{}> on non-writable memory", type_name!(T));
		&mut (*self.as_raw_ptr_slice(ofs, count))[..]
	}
	pub fn as_mut_slice<T: crate::lib::POD>(&mut self, ofs: usize, count: usize) -> &mut [T]
	{
		assert!( self.is_mutable(),
			"Calling as_mut_slice<{}> on non-writable memory", type_name!(T) );
		// SAFE: &mut and Plain-old-data
		unsafe {
			self.as_int_mut_slice(ofs, count)
		}
	}
	pub fn into_array<T: crate::lib::POD>(self) -> ArrayHandle<T>
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
		write!(f, "{:p}+{}pg ({})", self.addr(), self.count(), if self.is_mutable() { "mut" } else { "ro" })
	}
}
impl Drop for AllocHandle
{
	fn drop(&mut self)
	{
		if self.count() > 0
		{
			// SAFE: Dropping an allocation controlled by this object
			unsafe { unmap(self.addr() as *mut (), self.count()); }
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
//	fn as_ref(&self) -> &[u8] { self.alloc.as_slice(self.idx * PAGE_SIZE, PAGE_SIZE) }
//}
//impl<'a> ::core::convert::AsMut<[u8]> for PageHandle<'a>
//{
//	fn as_mut(&mut self) -> &mut [u8] { self.alloc.as_mut_slice(self.idx * PAGE_SIZE, PAGE_SIZE) }
//}

impl<T: crate::lib::POD> SliceAllocHandle<T>
{
}

impl<T: crate::lib::POD> ::core::ops::Deref for SliceAllocHandle<T>
{
	type Target = [T];
	fn deref(&self) -> &[T]
	{
		self.alloc.as_slice(self.ofs, self.count)
	}
}

impl<T: crate::lib::POD> ArrayHandle<T>
{
	pub fn len(&self) -> usize {
		self.alloc.len() / ::core::mem::size_of::<T>()
	}
}
impl<T: crate::lib::POD> ::core::ops::Deref for ArrayHandle<T>
{
	type Target = [T];
	fn deref(&self) -> &[T] {
		self.alloc.as_slice(0, self.len())
	}
}
impl<T: crate::lib::POD> ::core::ops::DerefMut for ArrayHandle<T>
{
	fn deref_mut(&mut self) -> &mut [T] {
		let len = self.len();
		self.alloc.as_mut_slice(0, len)
	}
}

pub use crate::arch::memory::virt::AddressSpace;

// vim: ft=rust
