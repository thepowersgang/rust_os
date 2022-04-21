//
//
//
//! 
use super::{ZERO_ALLOC, Error};
use crate::arch::memory::addresses;

// Curse no CTFE
//const HEADERS_SIZE: usize = ::core::mem::size_of::<HeapHead>() + ::core::mem::size_of::<HeapFoot>();
const MAGIC: u32 = 0x71ff11A1;

// TODO: Store the limits in the definition
pub struct HeapDef
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



impl HeapDef
{
	/// Construct a new heap instance
	pub const fn new() -> HeapDef {
		HeapDef {
			start: 0 as *mut _,
			last_foot: 0 as *mut _,
			first_free: 0 as *mut _,
			}
	}

	/// Allocate arbitary bytes from the heap
	/// 
	// TODO: Is this actually unsafe?
	pub unsafe fn allocate(&mut self, size: usize, align: usize) -> Result<*mut (), Error>
	{
		// TODO: Have different pools for different alignments
		
		// SHORT CCT: Zero size allocation
		if size == 0 {
			return Ok(ZERO_ALLOC);
		}
		
		// This would be static, if CTFE was avalible
		let headers_size = ::core::mem::size_of::<HeapHead>() + ::core::mem::size_of::<HeapFoot>();
		
		// 1. Round size up to closest heap block size
		let blocksize = crate::lib::num::round_up(size + headers_size, 32);
		log_debug!("allocate(size={},align={}) blocksize={}", size, align, blocksize);

		// 2. Locate a free location
		// Check all free blocks for one that would fit this allocation
		let mut prev = ::core::ptr::null_mut();
		let mut opt_fb = self.first_free;
		while !opt_fb.is_null()
		{
			let fb = &*opt_fb;
			assert!( fb.magic == MAGIC );
			let next = match fb.state
				{
				HeapState::Free(n) => n,
				_ => {
					self.dump();
					log_error!("Non-free block ({:p}) in free list", fb);
					return Err(Error::Corrupted)
					}
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
			let next = match fb.state { HeapState::Free(n) => n, _ => unreachable!() };
			// Split block (if needed)
			if fb.size() > blocksize + headers_size
			{
				let far_foot = fb.foot() as *mut HeapFoot;
				let far_size = fb.size() - blocksize;
				fb.resize(blocksize);
				
				let far_head = fb.next();
				assert!(far_head != prev);
				(*far_head).initialise( far_size, HeapState::Free(next) );
				assert_eq!( far_foot, (*far_head).foot() as *mut _ );

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
			return Ok( fb.data() );
		}
		assert!(opt_fb.is_null());

		// Fall through: No free blocks would fit the allocation
		
		// 3. If none, allocate more space
		let block_ptr = self.expand(blocksize)?;
		let block = &mut *block_ptr;
		// > Split returned block into a block of required size and a free block
		if block.size() > blocksize
		{
			// Create a new block header at end of block
			let tailsize = block.size() - blocksize;
			block.resize(blocksize);

			(*block.next()).initialise( tailsize, HeapState::Free(self.first_free) );

			self.first_free = block.next();
		}
		
		block.state = HeapState::Used(size);	
	
		log_trace!("Returning block {:p} (new)", block);
		Ok( block.data() )
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
		// TODO: Can this expand into the next block?
		else
		{
			false
		}
	}
	pub unsafe fn shrink_alloc(&mut self, ptr: *mut (), new_size: usize)
	{
		//let headers_size = ::core::mem::size_of::<HeapHead>() + ::core::mem::size_of::<HeapFoot>();
		
		// Quick check: Zero allocation can't grow or shrink
		if ptr == ZERO_ALLOC {
			assert!(new_size == 0, "Calling shrink_alloc on ZERO_ALLOC with new_size!=0");
			return ;
		}
		
		let headptr = {
			let hp = (ptr as *mut HeapHead).offset(-1);
			assert!( (hp as usize) >= self.start as usize );
			assert!( (hp as usize) < self.last_foot as usize );
			&mut *hp
			};

		match headptr.state
		{
		HeapState::Used(ref mut sz) => {
			// TODO: Split block if possible
			assert!(*sz >= new_size, "Calling shrink_alloc with a larger size");
			*sz = new_size;
			},
		HeapState::Free(..) => panic!("Calling shrink_alloc on a free block ({:p})", ptr),
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
			headref.validate().unwrap();
			if size == 0 {
				// Special case for use as C free() (as part of ACPICA shim)
				assert!( is!(headref.state, HeapState::Used(_)), "Header {:p} state invalid {:?} not Used(_)",
					headref, headref.state );
			}
			else {
				assert_eq!( headref.state, HeapState::Used(size), "Header {:p} state invalid {:?} not Used({})",
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
	unsafe fn expand(&mut self, min_size: usize) -> Result<*mut HeapHead, Error>
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
			};
		//log_debug!("(2) self.{{start = {:#x}, last_foot = {:?}}}, use_prev={}", self.start as usize, self.last_foot, use_prev);
		assert!( !self.last_foot.is_null() );
		let last_foot = &mut *self.last_foot;
		let alloc_size = min_size - (if use_prev { last_foot.head().size() } else { 0 });
		
		
		// 1. Allocate at least one page at the end of the heap
		let n_pages = crate::lib::num::round_up(alloc_size, crate::PAGE_SIZE) / crate::PAGE_SIZE;
		//log_debug!("HeapDef.expand(min_size={}), alloc_size={}, n_pages={}", min_size, alloc_size, n_pages);
		//log_trace!("last_foot = {:p}", self.last_foot);
		assert!(n_pages > 0);

		if last_foot.next_head() as usize == addresses::HEAP_END {
			return Err( Error::OutOfReservation );
		}
		if last_foot.next_head() as usize + n_pages * crate::PAGE_SIZE > addresses::HEAP_END {
			return Err( Error::OutOfReservation );
		}

		// Allocate memory into the new region
		match crate::memory::virt::allocate(last_foot.next_head() as *mut(), n_pages)
		{
		Ok(_) => {},
		Err(crate::memory::virt::MapError::OutOfMemory) => return Err(Error::OutOfMemory),
		Err(e @ _) => panic!("Unknown error from VMM: {:?}", e),
		}

		// 2. If the final block is a free block, expand it to cover the new area
		let block = if use_prev
			{
				let block = &mut *last_foot.head;
				log_debug!("HeapDef.expand: (prev) &block={:p}", block);
				let newsize = block.size() + n_pages * crate::PAGE_SIZE;
				block.resize(newsize);
				
				block
			}
			else
			{
				let block = &mut *last_foot.next_head();

				log_debug!("HeapDef.expand: (new) &block={:p}", block);
				(*block).initialise( n_pages * crate::PAGE_SIZE, HeapState::Free(0 as *mut _) );
				
				block
			};
		self.last_foot = block.foot() as *mut HeapFoot;
		//log_debug!("HeapDef.expand: &block={:p}, self.last_foot={:p}", block, self.last_foot);
		//block.state = HeapState::Used(0);
		// 3. Return final block
		Ok( block )
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
				if head_ref.foot_im() as *const HeapFoot == self.last_foot {
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
	
	fn ptr(&self) -> *mut HeapHead { self as *const _ as *mut _ }
	pub unsafe fn prev(&self) -> *mut HeapHead
	{
		(*(self.ptr() as *mut HeapFoot).offset(-1)).head()
	}

	/// Obtain a raw pointer to the next block linearly
	pub fn next(&self) -> *mut HeapHead
	{
		// SAFE: This block should control its entire allocation, thus the offsetting is valid
		unsafe { 
			(self.ptr() as *mut u8).offset( self.size as isize ) as *mut HeapHead
		}
	}

	pub unsafe fn data(&mut self) -> *mut ()
	{
		self.ptr().offset( 1 ) as *mut ()
	}

	pub fn foot(&mut self) -> &mut HeapFoot
	{
		// SAFE: Block foot is owned by the block
		unsafe {
			&mut *(self.next() as *mut HeapFoot).offset( -1 )
		}
	}
	pub fn foot_im(&self) -> &HeapFoot
	{
		// SAFE: Block foot is owned by the block
		unsafe {
			& *(self.next() as *const HeapFoot).offset( -1 )
		}
	}

	pub fn validate(&mut self) -> Result<(), Error>
	{
		let selfptr: *mut _ = self;
		if self.magic != MAGIC {
			log_error!("Header {:p} magic invalid {:#x} instead of {:#x}",  self, self.magic, MAGIC );
			return Err( Error::Corrupted );
		}
		let foot_headptr: *mut _ = self.foot().head();
		if foot_headptr != selfptr {
			log_error!("Header {:p} foot backlink invalid, {:p} points to {:p}",  selfptr, self.foot_im(), foot_headptr );
			return Err( Error::Corrupted );
		}
		Ok( () )
	}

	pub unsafe fn resize(&mut self, new_size: usize)
	{
		assert!( is!(self.state, HeapState::Free(_)) );
		self.size = new_size;
		*self.foot() = HeapFoot {
				head: self as *mut _,
				};
	}

	/// UNSAFE: Assumes that the passed size is valid
	pub unsafe fn initialise(&mut self, size: usize, state: HeapState)
	{
		*self = HeapHead {
			magic: MAGIC,
			state: state,
			size: size,
			};
		self.foot().head = self;
	}
}

impl HeapFoot
{
	pub fn head(&mut self) -> &mut HeapHead
	{
		// SAFE: It's your own dumb fault if this is somewhere invalid
		unsafe {
			&mut *self.head
		}
	}
	pub unsafe fn next_head(&mut self) -> *mut HeapHead
	{
		let self_ptr: *mut HeapFoot = self;
		self_ptr.offset(1) as *mut HeapHead
	}
}
