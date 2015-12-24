// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/heap.rs
// - Dynamic memory manager

// TODO: Rewrite this to correctly use the size information avaliable

use core::ptr::Unique;
use core::ops;
use arch::memory::addresses;

// --------------------------------------------------------
// Types
#[derive(Copy,Clone)]
pub enum HeapId
{
	Local,	// Inaccessible outside of process
	Global,	// Global allocations
}

struct HeapDef
{
	start: *mut HeapHead,
	last_foot: *mut HeapFoot,
	first_free: *mut HeapHead,
}
unsafe impl ::core::marker::Send for HeapDef {}

#[derive(Debug,PartialEq)]	// RawPtr Debug is the address
enum HeapState
{
	Free(*mut HeapHead),
	Used(usize),
}

struct HeapHead
{
	magic: u32,
	size: usize,
	state: HeapState,
}
impl_fmt! {
	Debug(self, f) for HeapHead {
		write!(f, "HeapHead {{ magic: {:#x}, size: {:#x}, state: {:?} }}", self.magic, self.size, self.state)
	}
}
struct HeapFoot
{
	head: *mut HeapHead,
}

//pub struct AnyAlloc
//{
//	ptr: Unique<()>,
//}
//pub struct TypedAlloc<T>
//{
//	ptr: Unique<T>,
//}
pub struct ArrayAlloc<T>
{
	ptr: Unique<T>,
	count: usize,
}

// Curse no CTFE
//const HEADERS_SIZE: usize = ::core::mem::size_of::<HeapHead>() + ::core::mem::size_of::<HeapFoot>();
const MAGIC: u32 = 0x71ff11A1;
pub const ZERO_ALLOC: *mut () = 1 as *mut _;
// --------------------------------------------------------
// Globals
//#[link_section(process_local)] static s_local_heap : ::sync::Mutex<HeapDef> = mutex_init!(HeapDef{head:None});
#[allow(non_upper_case_globals)]
static s_global_heap : ::sync::Mutex<HeapDef> = mutex_init!(HeapDef{start:0 as*mut _, last_foot:0 as*mut _, first_free:0 as*mut _});

// --------------------------------------------------------
// Code
pub fn init()
{
}

// Used by Box<T>
#[lang="exchange_malloc"]
unsafe fn exchange_malloc(size: usize, align: usize) -> *mut u8
{
	match allocate(HeapId::Global, size, align)
	{
	Some(x) => x as *mut u8,
	None => panic!("exchange_malloc({}, {}) out of memory", size, align),
	}
}
#[lang="exchange_free"]
unsafe fn exchange_free(ptr: *mut u8, size: usize, align: usize)
{
	s_global_heap.lock().deallocate(ptr as *mut (), size, align)
}

// Used by libgcc
#[no_mangle] pub unsafe extern "C" fn malloc(size: usize) -> *mut () {
	allocate(HeapId::Global, size, 16).unwrap()
} 
#[no_mangle] pub unsafe extern "C" fn free(ptr: *mut ()) {
	if !ptr.is_null() {
		deallocate(ptr, 0, 16)
	}
} 

// Used by kernel internals
pub unsafe fn alloc<T>(value: T) -> *mut T
{
	let ret = match allocate(HeapId::Global, ::core::mem::size_of::<T>(), ::core::mem::align_of::<T>())
		{
		Some(v) => v as *mut T,
		None => panic!("Out of memory")
		};
	::core::ptr::write(ret, value);
	ret
}
pub unsafe fn alloc_raw(size: usize, align: usize) -> *mut () {
	match allocate(HeapId::Global, size, align)
	{
	Some(v) => v,
	None => panic!("Out of memory")
	}
}
pub unsafe fn dealloc<T>(value: *mut T)
{
	deallocate(value as *mut (), ::core::mem::size_of::<T>(), ::core::mem::align_of::<T>());
}
pub unsafe fn dealloc_raw(ptr: *mut (), size: usize, align: usize) {
	deallocate(ptr, size, align);
}

impl<T> ArrayAlloc<T>
{
	pub fn new(count: usize) -> ArrayAlloc<T>
	{
		// SAFE: Correctly constructs 'Unique' instances
		unsafe {
			if ::core::mem::size_of::<T>() == 0 {
				ArrayAlloc { ptr: Unique::new(ZERO_ALLOC as *mut T), count: !0 }
			}
			else if count == 0 {
				ArrayAlloc { ptr: Unique::new(ZERO_ALLOC as *mut T), count: 0 }
			}
			else
			{
				let ptr = match allocate(HeapId::Global, ::core::mem::size_of::<T>() * count, ::core::mem::align_of::<T>())
					{
					Some(v) => v as *mut T,
					None => panic!("Out of memory when allocating array of {} elements", count)
					};
				assert!(!ptr.is_null());
				ArrayAlloc { ptr: Unique::new(ptr), count: count }
			}
		}
	}
	pub unsafe fn from_raw(ptr: *mut T, count: usize) -> ArrayAlloc<T> {
		ArrayAlloc { ptr: Unique::new(ptr), count: count }
	}
	pub fn into_raw(self) -> *mut [T] {
		let ptr = *self.ptr;
		let count = self.count;
		::core::mem::forget(self);
		// SAFE: Takes ownership
		unsafe {
			::core::slice::from_raw_parts_mut(ptr, count)
		}
	}
	
	pub fn count(&self) -> usize { self.count }
	
	pub fn get_base(&self) -> *const T { *self.ptr }
	pub fn get_base_mut(&mut self) -> *mut T { *self.ptr }
	
	#[tag_safe(irq)]
	pub fn get_ptr_mut(&mut self, idx: usize) -> *mut T {
		// SAFE: Index asserted to be valid, have &mut
		unsafe {
			assert!(idx < self.count, "ArrayAlloc<{}>::get_mut({}) OOB {}", type_name!(T), idx, self.count);
			self.ptr.offset(idx as isize)
		}
	}
	#[tag_safe(irq)]
	pub fn get_ptr(&self, idx: usize) -> *const T {
		// SAFE: Index asserted to be valid
		unsafe {
			assert!(idx < self.count, "ArrayAlloc<{}>::get_ptr({}) OOB {}", type_name!(T), idx, self.count);
			self.ptr.offset(idx as isize)
		}
	}

	/// Attempt to expand this array without reallocating	
	pub fn expand(&mut self, new_count: usize) -> bool
	{
		if new_count > self.count
		{
			let newsize = ::core::mem::size_of::<T>() * new_count;
			// SAFE: Pointer is valid
			if unsafe { expand( *self.ptr as *mut(), newsize ) }
			{
				self.count = new_count;
				true
			}
			else
			{
				false
			}
		}
		else
		{
			log_warning!("ArrayAlloc<{}>::expand: Called with <= count", type_name!(T));
			true
		}
	}
	
	pub fn shrink(&mut self, new_count: usize)
	{
		// TODO: 
		log_warning!("TODO: ArrayAlloc::shrink");
	}
}
impl_fmt!{
	<T> Debug(self,f) for ArrayAlloc<T> {
		write!(f, "ArrayAlloc {{ {:p} + {} }}", *self.ptr, self.count)
	}
}
impl<T> ops::Drop for ArrayAlloc<T>
{
	fn drop(&mut self)
	{
		if self.count > 0 {
			// SAFE: Pointer is valid
			unsafe { deallocate(*self.ptr as *mut (), ::core::mem::size_of::<T>() * self.count, ::core::mem::align_of::<T>()) };
		}
	}
}

// Main entrypoints
unsafe fn allocate(heap: HeapId, size: usize, align: usize) -> Option<*mut ()>
{
	match heap
	{
	HeapId::Global => s_global_heap.lock().allocate(size, align),
	_ => panic!("TODO: Non-global heaps"),
	}
}

/// Attempt to expand in-place
unsafe fn expand(pointer: *mut (), newsize: usize) -> bool
{
	s_global_heap.lock().expand_alloc(pointer, newsize)
}

unsafe fn deallocate(pointer: *mut (), size: usize, align: usize)
{
	s_global_heap.lock().deallocate(pointer as *mut (), size, align);
}

impl HeapDef
{
	pub unsafe fn allocate(&mut self, size: usize, align: usize) -> Option<*mut ()>
	{
		// Have different pools for different alignments
		
		// SHORT CCT: Zero size allocation
		if size == 0 {
			return Some(ZERO_ALLOC);
		}
		
		// This would be static, if CTFE was avalible
		let headers_size = ::core::mem::size_of::<HeapHead>() + ::core::mem::size_of::<HeapFoot>();
		
		// 1. Round size up to closest heap block size
		let blocksize = ::lib::num::round_up(size + headers_size, 32);
		log_debug!("allocate(size={},align={}) blocksize={}", size, align, blocksize);
		// 2. Locate a free location
		// Check all free blocks for one that would fit this allocation
		let mut prev = ::core::ptr::null_mut();
		let mut opt_fb = self.first_free;
		while !opt_fb.is_null()
		{
			let fb = &*opt_fb;
			assert!( fb.magic == MAGIC );
			let next = match fb.state {
				HeapState::Free(n)=> n,
				_ => { self.dump(); panic!("Non-free block ({:p}) in free list", opt_fb); }
				};
			if fb.size() >= blocksize
			{
				break;
			}
			prev = opt_fb;
			assert!(opt_fb != next);
			opt_fb = next;
		}
		if !opt_fb.is_null()
		{
			let fb = &mut *opt_fb;
			let next = match fb.state { HeapState::Free(n)=> n, _ => {self.dump(); panic!("Non-free block in free list"); } };
			// Split block (if needed)
			if fb.size() > blocksize + headers_size
			{
				let far_foot = fb.foot() as *mut HeapFoot;
				let far_size = fb.size() - blocksize;
				fb.resize(blocksize);
				
				let far_head = fb.next();
				assert!(far_head != prev);
				*far_head = HeapHead {
					magic: MAGIC,
					size: far_size,
					state: HeapState::Free(next)
					};
				(*far_foot).head = far_head;
				log_debug!("Split block, new block {:?}", *far_head);
				if prev.is_null() {
					self.first_free = far_head;
				}
				else {
					(*prev).state = HeapState::Free(far_head);
					log_debug!("Chained atop {:p} {:?}", prev, *prev);
				}
			}
			else if prev.is_null()
			{
				self.first_free = next;
				log_debug!("Set first free to {:p}", next);
			}
			else
			{
				(*prev).state = HeapState::Free(next);
				log_debug!("Chain with next with {:p} {:?}", prev, *prev);
			}
			// Return newly allocated block
			fb.state = HeapState::Used(size);
			log_debug!("Returning block {:p} (Freelist)", fb);
			return Some( fb.data() );
		}
		assert!(opt_fb.is_null());
		// Fall: No free blocks would fit the allocation
		//log_trace!("allocate - No suitable free blocks");
		
		// 3. If none, allocate more space
		let block_ptr = self.expand(blocksize);
		let block = &mut *block_ptr;
		// > Split returned block into a block of required size and a free block
		if block.size() > blocksize
		{
			// Create a new block header at end of block
			let tailsize = block.size() - blocksize;
			block.resize(blocksize);
			let tailblock = &mut *block.next();
			*tailblock = HeapHead {
				magic: MAGIC,
				size: tailsize,
				state: HeapState::Free(self.first_free),
				};
			tailblock.foot().head = block.next();
			self.first_free = block.next();
		}
		
		block.state = HeapState::Used(size);	
	
		log_trace!("Returning block {:p} (new)", block);
		Some( block.data() )
	}

	/// Attempt to expand the specified block without reallocating
	pub unsafe fn expand_alloc(&mut self, ptr: *mut (), size: usize) -> bool
	{
		let headers_size = ::core::mem::size_of::<HeapHead>() + ::core::mem::size_of::<HeapFoot>();
		
		// Quick check: Zero allocation can't grow
		if ptr == ZERO_ALLOC {
			return false;
		}
		
		let headptr = {
			let hp = (ptr as *mut HeapHead).offset(-1);
			assert!( (hp as usize) >= self.start as usize );
			assert!( (hp as usize) < self.last_foot as usize );
			&mut *hp
			};

		// If the new size fits within the old block, update the cached size and return true
		if size + headers_size <= headptr.size()
		{
			headptr.state = HeapState::Used(size);
			true
		}
		else
		{
			false
		}
	}
	
	pub unsafe fn deallocate(&mut self, ptr: *mut (), size: usize, _align: usize)
	{
		log_debug!("deallocate(ptr={:p},size={:#x})", ptr, size);
		if ptr == ZERO_ALLOC {
			assert!(size == 0, "ZERO_ALLOC but size({}) != 0", size);
			log_trace!("Free zero alloc");
			return ;
		}

		let mut no_add = false;
		let headptr = (ptr as *mut HeapHead).offset(-1);
		assert!(headptr as usize >= addresses::HEAP_START);
		
		{
			let headref = &mut *headptr;
			assert!( headref.magic == MAGIC, "Header {:p} magic invalid {:#x} instead of {:#x}",
				headref, headref.magic, MAGIC );
			assert!( headref.foot().head() as *mut _ == headptr, "Header {:p} foot backlink invalid, {:p} points to {:p}",
				headref, headref.foot(), headref.foot().head() );
			if size == 0 {
				// Special case for use as C free() (as part of ACPICA shim)
				assert!( is!(headref.state, HeapState::Used(_)), "Header {:p} state invalid {:?} not Used(_)",
					headref, headref.state );
			}
			else {
				assert!( headref.state == HeapState::Used(size), "Header {:p} state invalid {:?} not Used({:#x})",
					headref, headref.state, size );
			}
			
			// Merge left and right
			// 1. Left:
			if headptr as usize != addresses::HEAP_START
			{
				if let HeapState::Free(_) = (*headref.prev()).state
				{
					log_trace!("Merged left with {:p}", headref.prev());
					// increase size of previous block to cover this block
					let prev_block = &mut *headref.prev();
					let new_size = prev_block.size() + headref.size();
					prev_block.resize( new_size );
					no_add = true;
				}
			}
			
			// 2. Right
			//if_let!( HeapState::Free(_) => 
			// TODO: Merging right requires being able to arbitarily remove items from the free list
		}
		
		if !no_add
		{
			(*headptr).state = HeapState::Free(self.first_free);
			self.first_free = headptr;
		}
	}
	
	/// Expand the heap to create a block at least `min_size` bytes long at the end
	/// \return New block, pre-allocated
	#[inline(never)]
	unsafe fn expand(&mut self, min_size: usize) -> *mut HeapHead
	{
		log_trace!("HeapDef::expand(min_size={:#x})", min_size);
		//log_debug!("self.{{start = {:p}, last_foot = {:?}}}", self.start, self.last_foot);
		let use_prev =
			if self.start.is_null() {
				let base = addresses::HEAP_START;
				self.start = base as *mut HeapHead;
				// note: Evil hack, set last_foot to invalid memory (it's only used for .next_head())
				self.last_foot = (base as *mut HeapFoot).offset(-1);
				false
			}
			else {
				false
				// DISABLED: To use the final block, the previous block on the freelist must be edited
				// - Not easy to do
				//assert!(!self.last_foot.is_null());
				//let lasthdr = (*self.last_foot).head();
				//match lasthdr.state
				//{
				//HeapState::Free(_) => {
				//	assert!(lasthdr.size < min_size);
				//	true
				//	},
				//HeapState::Used(_) => false
				//}
			};
		//log_debug!("(2) self.{{start = {:#x}, last_foot = {:?}}}, use_prev={}", self.start as usize, self.last_foot, use_prev);
		assert!( !self.last_foot.is_null() );
		let last_foot = &mut *self.last_foot;
		let alloc_size = min_size - (if use_prev { last_foot.head().size() } else { 0 });
		
		// 1. Allocate at least one page at the end of the heap
		let n_pages = ::lib::num::round_up(alloc_size, ::PAGE_SIZE) / ::PAGE_SIZE;
		//log_debug!("HeapDef.expand(min_size={}), alloc_size={}, n_pages={}", min_size, alloc_size, n_pages);
		//log_trace!("last_foot = {:p}", self.last_foot);
		assert!(n_pages > 0);
		::memory::virt::allocate(last_foot.next_head() as *mut(), n_pages);
		
		// 2. If the final block is a free block, allocate it and expand to cover the new area
		let block = if use_prev
			{
				let block = &mut *last_foot.head;
				log_debug!("HeapDef.expand: (prev) &block={:p}", block);
				let newsize = block.size() + n_pages * ::PAGE_SIZE;
				block.resize(newsize);
				
				block
			}
			else
			{
				let block = &mut *last_foot.next_head();
				log_debug!("HeapDef.expand: (new) &block={:p}", block);
				*block = HeapHead {
					magic: MAGIC,
					state: HeapState::Free(0 as *mut _),
					size: n_pages * ::PAGE_SIZE,
					};
				block.foot().head = last_foot.next_head();
				
				block
			};
		self.last_foot = block.foot() as *mut HeapFoot;
		//log_debug!("HeapDef.expand: &block={:p}, self.last_foot={:p}", block, self.last_foot);
		//block.state = HeapState::Used(0);
		// 3. Return final block
		block
	}
	
	fn dump(&self)
	{
		log_log!("Dumping Heap");
		// SAFE: Does an immutable heap walk
		unsafe {
			let mut block_head = self.start;
			loop
			{
				let head_ref = &*block_head;
				log_log!("{:p} {:?}", block_head, head_ref);
				if head_ref.foot() as *const HeapFoot == self.last_foot {
					break;
				}
				block_head = head_ref.next();
			}
		}
	}
}

impl HeapHead
{
	fn size(&self) -> usize { self.size }
	
	unsafe fn ptr(&self) -> *mut HeapHead { ::core::mem::transmute(self) }
	pub unsafe fn prev(&self) -> *mut HeapHead
	{
		(*(self.ptr() as *mut HeapFoot).offset(-1)).head()
	}
	pub unsafe fn next(&self) -> *mut HeapHead
	{
		(self.ptr() as *mut u8).offset( self.size as isize ) as *mut HeapHead
	}
	pub unsafe fn data(&mut self) -> *mut ()
	{
		self.ptr().offset( 1 ) as *mut ()
	}
	pub unsafe fn foot<'a>(&'a self) -> &'a mut HeapFoot
	{
		::core::mem::transmute( (self.next() as *mut HeapFoot).offset( -1 ) )
	}
	
	pub unsafe fn resize(&mut self, new_size: usize)
	{
		assert!( is!(self.state, HeapState::Free(_)) );
		self.size = new_size;
		*self.foot() = HeapFoot {
				head: self as *mut _,
				};
	}
}

impl HeapFoot
{
	pub unsafe fn head<'a>(&'a mut self) -> &'a mut HeapHead
	{
		::core::mem::transmute( self.head )
	}
	pub unsafe fn next_head(&mut self) -> *mut HeapHead
	{
		let self_ptr: *mut HeapFoot = ::core::mem::transmute(self);
		self_ptr.offset(1) as *mut HeapHead
	}
}

// vim: ft=rust
