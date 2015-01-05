// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/heap.rs
// - Dynamic memory manager

use core::option::Option::{self,None,Some};
use core::ptr::PtrExt;

// --------------------------------------------------------
// Types
#[derive(Copy)]
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
unsafe impl ::core::kinds::Send for HeapDef {}

#[allow(raw_pointer_deriving)]
#[derive(Show)]	// RawPtr Show is the address
enum HeapState
{
	Free(*mut HeapHead),
	Used(uint),
}

#[derive(Show)]
struct HeapHead
{
	magic: uint,
	size: uint,
	state: HeapState,
}
struct HeapFoot
{
	head: *mut HeapHead,
}

// Curse no CTFE
//const HEADERS_SIZE: uint = ::core::mem::size_of::<HeapHead>() + ::core::mem::size_of::<HeapFoot>();
const MAGIC: uint = 0x71ff11A1;
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

#[lang="exchange_malloc"]
unsafe fn exchange_malloc(size: uint, _align: uint) -> *mut u8 {
	allocate(HeapId::Global, size).unwrap() as *mut u8
}
#[lang="exchange_free"]
unsafe fn exchange_free(ptr: *mut u8, _size: uint, _align: uint) {
	deallocate(ptr)
}

pub unsafe fn alloc<T>(value: T) -> *mut T
{
	let ret = match allocate(HeapId::Global, ::core::mem::size_of::<T>())
		{
		Some(v) => v as *mut T,
		None => panic!("Out of memory")
		};
	::core::ptr::write(ret, value);
	ret
}

pub unsafe fn alloc_array<T>(count: uint) -> *mut T
{
	match allocate(HeapId::Global, ::core::mem::size_of::<T>() * count)
	{
	Some(v) => v as *mut T,
	None => panic!("Out of memory when allocating array of {} elements", count)
	}
}

pub unsafe fn allocate(heap: HeapId, size: uint) -> Option<*mut ()>
{
	match heap
	{
	HeapId::Global => s_global_heap.lock().allocate(size),
	_ => panic!("TODO: Non-global heaps"),
	}
}

//pub unsafe fn expand(pointer: *mut (), newsize: uint) -> Option<*mut ()>
//{
//	panic!("TODO: heap::expand");
//	None
//}

pub unsafe fn deallocate<T>(pointer: *mut T)
{
	s_global_heap.lock().deallocate(pointer as *mut ());
}

impl HeapDef
{
	pub unsafe fn allocate(&mut self, size: uint) -> Option<*mut ()>
	{
		// SHORT CCT: Zero size allocation
		if size == 0 {
			return Some(ZERO_ALLOC);
		}
		
		// This would be static, if CTFE was avalible
		let headers_size = ::core::mem::size_of::<HeapHead>() + ::core::mem::size_of::<HeapFoot>();
		
		// 1. Round size up to closest heap block size
		let blocksize = ::lib::num::round_up(size + headers_size, 32);
		log_debug!("allocate(size={}) blocksize={}", size, blocksize);
		// 2. Locate a free location
		// Check all free blocks for one that would fit this allocation
		let mut prev = ::core::ptr::null_mut();
		let mut opt_fb = self.first_free;
		while !opt_fb.is_null()
		{
			let fb = &*opt_fb;
			assert!( fb.magic == MAGIC );
			let next = match fb.state { HeapState::Free(n)=> n, _ => panic!("Non-free block ({}) in free list", opt_fb) };
			if fb.size >= blocksize
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
			let next = match fb.state { HeapState::Free(n)=> n, _ => panic!("Non-free block in free list") };
			// Split block (if needed)
			if fb.size > blocksize + headers_size
			{
				let far_foot = fb.foot() as *mut _;
				let far_size = fb.size - blocksize;
				fb.size = blocksize;
				*fb.foot() = HeapFoot {
					head: fb as *mut _,
					};
				
				let far_head = fb.next();
				assert!(far_head != prev);
				*far_head = HeapHead {
					magic: MAGIC,
					size: far_size,
					state: HeapState::Free(next)
					};
				(*far_foot).head = far_head;
				if prev.is_null() {
					self.first_free = far_head;
				}
				else {
					(*prev).state = HeapState::Free(far_head);
				}
			}
			else
			{
				let next = match fb.state { HeapState::Free(x) => x, _ => panic!("") };
				if prev.is_null() {
					self.first_free = next;
				}
				else {
					(*prev).state = HeapState::Free(next);
				}
			}
			// Return newly allocated block
			fb.state = HeapState::Used(size);
			log_debug!("Returning {} (Freelist)", fb.data());
			return Some( fb.data() );
		}
		assert!(opt_fb.is_null());
		// Fall: No free blocks would fit the allocation
		//log_trace!("allocate - No suitable free blocks");
		
		// 3. If none, allocate more space
		let block_ptr = self.expand(blocksize);
		let block = &mut *block_ptr;
		// > Split returned block into a block of required size and a free block
		if block.size > blocksize
		{
			// Create a new block header at end of block
			let tailsize = block.size - blocksize;
			block.size = blocksize;
			*block.foot() = HeapFoot {
				head: block_ptr,
				};
			let tailblock = &mut *block.next();
			*tailblock = HeapHead {
				magic: MAGIC,
				size: tailsize,
				state: HeapState::Free(self.first_free),
				};
			tailblock.foot().head = block.next();
			self.first_free = block.next();
		}
		
		log_trace!("Returning {} (new)", block.data());
		Some( block.data() )
	}

	pub fn deallocate(&mut self, ptr: *mut ())
	{
		log_debug!("deallocate(ptr={})", ptr);
		if ptr == ZERO_ALLOC {
			log_trace!("Free zero alloc");
			return ;
		}
		unsafe
		{
			let mut no_add = false;
			let headptr = (ptr as *mut HeapHead).offset(-1);
			
			{
				let headref = &mut *headptr;
				assert!( headref.magic == MAGIC );
				assert!( headref.foot().head() as *mut _ == headptr );
				
				// Merge left and right
				// 1. Left:
				if_let!( HeapState::Free(_) = (*headref.prev()).state
				{
					log_trace!("Merged left with {}", headref.prev());
					// increase size of previous block to cover this block
					(*headref.prev()).size += headref.size;
					no_add = true;
				})
				
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
	}
	
	/// Expand the heap to create a block at least `min_size` bytes long at the end
	/// \return New block, pre-allocated
	unsafe fn expand(&mut self, min_size: uint) -> *mut HeapHead
	{
		let use_prev =
			if self.start.is_null() {
				let base = ::arch::memory::addresses::HEAP_START;
				self.start = base as *mut HeapHead;
				// note: Evil hack, set last_foot to invalid memory (it's only used for .next_head())
				self.last_foot = (base as *mut HeapFoot).offset(-1);
				false
			}
			else {
				assert!(!self.last_foot.is_null());
				let lasthdr = (*self.last_foot).head();
				match lasthdr.state
				{
				HeapState::Free(_) => {
					assert!(lasthdr.size < min_size);
					true
					},
				HeapState::Used(_) => false
				}
			};
		assert!( !self.last_foot.is_null() );
		let last_foot = &mut *self.last_foot;
		let alloc_size = min_size - (if use_prev { last_foot.head().size } else { 0 });
		
		// 1. Allocate at least one page at the end of the heap
		let n_pages = ::lib::num::round_up(alloc_size, ::PAGE_SIZE) / ::PAGE_SIZE;
		log_debug!("HeapDef.expand(min_size={}), alloc_size={}, n_pages={}", min_size, alloc_size, n_pages);
		assert!(n_pages > 0);
		::memory::virt::allocate(last_foot.next_head() as *mut(), n_pages);
		
		// 2. If the final block is a free block, allocate it and expand to cover the new area
		let block = if use_prev
			{
				let block = &mut *last_foot.head;
				log_debug!("HeapDef.expand: (prev) &block={}", block as *mut HeapHead);
				block.size += n_pages * ::PAGE_SIZE;
				block.foot().head = last_foot.head;
				
				block
			}
			else
			{
				let block = &mut *last_foot.next_head();
				log_debug!("HeapDef.expand: (new) &block={}", block as *mut HeapHead);
				*block = HeapHead {
					magic: MAGIC,
					state: HeapState::Used(0),
					size: n_pages * ::PAGE_SIZE,
					};
				block.foot().head = last_foot.next_head();
				
				block
			};
		self.last_foot = block.foot() as *mut HeapFoot;
		log_debug!("HeapDef.expand: &block={}", block as *mut HeapHead);
		block.state = HeapState::Used(0);
		// 3. Return final block
		block
	}
}

impl HeapHead
{
	unsafe fn ptr(&self) -> *mut HeapHead { ::core::mem::transmute(self) }
	pub unsafe fn prev(&self) -> *mut HeapHead
	{
		(*(self.ptr() as *mut HeapFoot).offset(-1)).head()
	}
	pub unsafe fn next(&self) -> *mut HeapHead
	{
		(self.ptr() as *mut u8).offset( self.size as int ) as *mut HeapHead
	}
	pub unsafe fn data(&mut self) -> *mut ()
	{
		self.ptr().offset( 1 ) as *mut ()
	}
	pub unsafe fn foot<'a>(&'a self) -> &'a mut HeapFoot
	{
		::core::mem::transmute( (self.next() as *mut HeapFoot).offset( -1 ) )
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
