// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/heap.rs
// - Dynamic memory manager

use core::option::{Option,None,Some};
use core::ptr::RawPtr;

// --------------------------------------------------------
// Types
enum HeapId
{
	LocalHeap,	// Inaccessible outside of process
	GlobalHeap,	// Global allocations
}

struct HeapDef
{
	start: Option<*mut HeapHead>,
	last_foot: Option<*mut HeapFoot>,
	first_free: Option<*mut HeapHead>,
}

enum HeapState
{
	HeapFree(Option<*mut HeapHead>),
	HeapUsed(uint),
}

struct HeapHead
{
	//magic: uint,
	size: uint,
	state: HeapState,
}
struct HeapFoot
{
	head: *mut HeapHead,
}

static MAGIC: uint = 0x71ff11A1;
// --------------------------------------------------------
// Globals
//#[link_section(process_local)] static s_local_heap : ::sync::Mutex<HeapDef> = mutex_init!(HeapDef{head:None});
static mut s_global_heap : ::sync::Mutex<HeapDef> = mutex_init!(HeapDef{start:None,last_foot:None,first_free:None});

// --------------------------------------------------------
// Code
pub fn init()
{
}

pub unsafe fn alloc<T>() -> *mut T
{
	match allocate(GlobalHeap, ::core::mem::size_of::<T>())
	{
	Some(v) => v as *mut T,
	None => fail!("Out of memory")
	}
}

pub unsafe fn allocate(heap: HeapId, size: uint) -> Option<*mut ()>
{
	s_global_heap.lock().allocate(size)
}

//pub unsafe fn expand(pointer: *mut (), newsize: uint) -> Option<*mut ()>
//{
//	fail!("TODO: heap::expand");
//	None
//}

pub unsafe fn deallocate(pointer: *mut ())
{
	fail!("TODO: heap::deallocate");
	
}

impl HeapDef
{
	pub unsafe fn allocate(&mut self, size: uint) -> Option<*mut ()>
	{
		// 1. Round size up to closest heap block size
		let blocksize = ::lib::num::round_up(size + ::core::mem::size_of::<HeapHead>() + ::core::mem::size_of::<HeapFoot>(), 32);
		log_debug!("allocate(size={}) blocksize={}", size, blocksize);
		// 2. Locate a free location
		if self.first_free.is_some()
		{
			log_trace!("allocate - Free blocks");
			// Check all free blocks for one that would fit this allocation
			let mut prev = None;
			let mut opt_fb = self.first_free;
			while opt_fb.is_some()
			{
				let fb = &mut *opt_fb.unwrap();
				let next = match fb.state { HeapFree(n)=> n, _ => fail!("Non-free block in free list") };
				if fb.size >= blocksize
				{
					break;
				}
				prev = opt_fb;
				opt_fb = next;
			}
			if opt_fb.is_some()
			{
				log_trace!("allocate - Suitable free block!");
				// Split block (if needed)
				// - Add split block to the free list
				// Return newly allocated block
			}
			// Fall: No free blocks would fit the allocation
			log_trace!("allocate - No suitable free blocks");
		}
		else
		{
			log_trace!("allocate - No free blocks");
		}
		
		// 3. If none, allocate more space
		let block_ptr = self.expand(blocksize);
		let block = &mut *block_ptr;
		// > Split returned block into a block of required size and a free block
		if block.size > blocksize
		{
			// Create a new block header at end of block
			let tailsize = block.size - blocksize;
			block.size = blocksize;
			block.foot().head = block_ptr;
			let tailblock = &mut *block.next();
			//tailblock.magic = MAGIC;
			tailblock.size = tailsize;
			tailblock.state = HeapFree(None);
			tailblock.foot().head = block.next();
			// Append to free list
		}
		
		Some( block.data() )
	}

	unsafe fn expand(&mut self, min_size: uint) -> *mut HeapHead
	{
		let use_prev =
			if self.start.is_none() {
				let base = ::arch::memory::addresses::heap_start;
				self.start = Some( base as *mut HeapHead );
				self.last_foot = Some( (base as *mut HeapFoot).offset(-1) );
				false
			}
			else {
				match (*self.last_foot.unwrap()).head().state
				{
				HeapFree(_) => true,
				HeapUsed(_) => false
				}
			};
		let last_foot = &mut *self.last_foot.unwrap();
		let alloc_size = min_size - (if use_prev { last_foot.head().size } else { 0 });
		// 1. Allocate at least one page at the end of the heap
		let n_pages = ::lib::num::round_up(alloc_size, ::PAGE_SIZE) / ::PAGE_SIZE;
		log_debug!("HeapDef.expand(min_size={}), n_pages={}", min_size, n_pages);
		assert!(n_pages > 0);
		::memory::virt::allocate(last_foot.next_head() as *mut(), n_pages);
		// 2. If the final block is a free block, allocate it and expand to cover the new area
		if use_prev
		{
			let block = &mut *last_foot.head;
			block.size += n_pages * ::PAGE_SIZE;
			block.state = HeapUsed(0);
			block.foot().head = last_foot.head;
		}
		
		// 3. Return final block
		fail!("TODO: heap::expand_heap");
	}
}

impl HeapHead
{
	unsafe fn ptr(&self) -> *mut HeapHead { ::core::mem::transmute(self) }
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
