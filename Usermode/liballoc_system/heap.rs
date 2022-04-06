//
//
//
use sync::Mutex;
use core::mem::{align_of,size_of};

use ::syscalls::PAGE_SIZE;
#[cfg(target_arch="x86_64")] const HEAP_LIMITS: (usize,usize) = (0x1000_0000_0000, 0x7000_0000_0000);
#[cfg(target_arch="arm")] const HEAP_LIMITS: (usize,usize) = (0x1000_0000, 0x7000_0000);
#[cfg(target_arch="aarch64")] const HEAP_LIMITS: (usize,usize) = (0x1000_0000, 0x7000_0000);
#[cfg(target_arch="riscv64")] const HEAP_LIMITS: (usize,usize) = (0x10_0000_0000, 0x38_0000_0000);


pub const EMPTY: *mut u8 = 1 as *mut u8;

pub static S_GLOBAL_HEAP: Mutex<AllocState> = Mutex::new(AllocState { start: 0 as *mut _, past_end: 0 as *mut _ } );

#[cfg(target_pointer_width="64")]
const PTR_SIZE: usize = 8;
#[cfg(target_pointer_width="32")]
const PTR_SIZE: usize = 4;

const MIN_BLOCK_SIZE: usize = 8 * PTR_SIZE;
const BLOCK_ALIGN: usize = 2 * PTR_SIZE;

pub fn get_usable_size(size: usize, _align: usize) -> (usize, usize)
{
	(size, size)
}

pub struct AllocState
{
	start: *mut Block,
	past_end: *mut Block,
//	first_free: BlockPointer,
}
unsafe impl Send for AllocState {}

#[derive(Debug)]
struct Block
{
	size: usize,
	state: BlockState,
}
#[derive(PartialEq,Debug)]
enum BlockState
{
	Free,
	Used(usize),
}
struct BlockTail
{
	head_ptr: *mut Block,
}

impl AllocState
{
	pub fn allocate(&mut self, size: usize, align: usize) -> Result<*mut (), ()>
	{
		kernel_log!("allocate");
		if size == 0 {
			kernel_log!("allocate({}, {}) = {:p}", size, align, EMPTY);
			return Ok( EMPTY as *mut () );
		}
		if self.start == self.past_end {
			self.extend_reservation(size);

			let block = self.last_block();
			let rv = block.allocate(size, align);
			kernel_log!("allocate({}, {}) = {:p}", size, align, rv);
			return Ok(rv);
		}
		
		for block in self.free_blocks()
		{
			// TODO: Block split
			if block.capacity(align) >= size
			{
				let rv = block.allocate(size, align);
				kernel_log!("allocate({}, {}) = {:p}", size, align, rv);
				return Ok(rv);
			}
		}

		let current_extra = self.last_block().self_free().map(|blk| blk.capacity(align)).unwrap_or(0);
		self.extend_reservation(size - current_extra);

		let block = self.last_block();
		let rv = block.allocate(size, align);
		kernel_log!("allocate({}, {}) = {:p}", size, align, rv);
		return Ok( rv );
	}
	/// Returns 'true' if expanding succeeded
	pub unsafe fn try_expand(&mut self, ptr: *mut (), size: usize, align: usize) -> Result<(),()> {
		if size == 0 {
			// TODO: Resize down to 0?
			Ok( () )
		}
		else if ptr == EMPTY as *mut () {
			Err( () )
		}
		else {
			let bp = Block::ptr_from_ptr(ptr, align);
			let bp = &mut *bp;
			if bp.capacity(align) > size {
				bp.state = BlockState::Used( size );
				kernel_log!("expand(bp={:p}, {}, {}) = true", bp, size, align);
				Ok( () )
			}
			else {
				let n = bp.next();
				if n == self.past_end {
					Err( () )
				}
				else if let Some(v) = (*n).self_free() {
					if bp.capacity(align) + v.size > size
					{
						// The next block is free, and has sufficient shared capacicty
						let new_size = bp.size + v.size;
						// - Resize this block to cover the next block (effectively deleting it)
						bp.initialise( new_size );
						// - Allocate this block again (potentially splitting it)
						bp.allocate( size, align );
						kernel_log!("expand(bp={:p}, {}, {}) = true", bp, size, align);

						Ok( () )
					}
					else
					{
						// Insufficient space in the next block
						Err( () )
					}
				}
				else {
					Err( () )
				}
			}
		}
	}
	pub unsafe fn deallocate(&mut self, ptr: *mut (), align: usize) {
		if ptr == EMPTY as *mut () {
			// Nothing needs to be done, as the allocation was empty
		}
		else {
			let bp = Block::ptr_from_ptr(ptr, align);
			let bp = &mut *bp;
			kernel_log!("deallocate(bp={:p}, align={})", bp, align);
			bp.state = BlockState::Free;

			let np = bp.next();
			if np == self.past_end {
				// Final block
			}
			else if let Some(next) = (*np).self_free() {
				let new_size = bp.size + next.size;
				bp.initialise( new_size );
			}
			else {
				// Next block isn't free, can't merge
			}
		}
	}

	fn last_block(&mut self) -> &mut Block {
		// SAFE: Mutable borrow prevents any form of aliasing
		unsafe { &mut *(*self.past_end).prev() }
	}
	fn extend_reservation(&mut self, required_space: usize) {
		let npages = (required_space + size_of::<Block>() + size_of::<BlockTail>() + PAGE_SIZE-1) / PAGE_SIZE;
		assert!(npages > 0);
		assert!(self.past_end != HEAP_LIMITS.1 as *mut Block);
		assert!(self.past_end as usize + (npages * PAGE_SIZE) <= HEAP_LIMITS.1);	// TODO: This isn't an assert conditon, it's an OOM
		if self.start.is_null() {
			self.start = HEAP_LIMITS.0 as *mut Block;
			self.past_end = HEAP_LIMITS.0 as *mut Block;
		}

		// SAFE: Allocates only in controlled region.
		let cb = unsafe {
			::syscalls::memory::allocate(self.past_end as usize, npages).expect("Heap allocation failure");
			(*self.past_end).initialise(npages * PAGE_SIZE);
			&mut *self.past_end
			};
		self.past_end = cb.next();

		if cb as *mut Block != self.start {
			// SAFE: Not the first block, and even if we were using a freelist, it wouldn't be a problem
			unsafe {
				cb.try_merge_left();
			}
		}
	}
	fn free_blocks(&mut self) -> FreeBlocks {
		FreeBlocks { cur: self.start, state: self, }
	}
}

struct FreeBlocks<'a>
{
	state: &'a mut AllocState,
	cur: *mut Block,
}
impl<'a> ::core::iter::Iterator for FreeBlocks<'a>
{
	type Item = &'a mut Block;
	fn next(&mut self) -> Option<Self::Item>
	{
		while self.cur != self.state.past_end
		{
			// SAFE: Only yields each block once (Block::next returns a rawptr, so doesn't invalidate this)
			let block = unsafe {
				let bp = self.cur;
				self.cur = (*self.cur).next();
				&mut *bp
				};
			//kernel_log!("FreeBlocks::next() - block={:p} {:?}", block, block);
			if let BlockState::Free = block.state {
				return Some(block);
			}
		}
		None
	}
}

impl Block
{ 
	unsafe fn ptr_from_ptr(ptr: *mut (), /*size: usize,*/ _align: usize) -> *mut Block {
		let bp_us = ptr as usize - size_of::<Block>();
		let bp = bp_us as *mut Block;
		//assert!( (*bp).state == BlockState::Used(size) );
		bp
	}

	/// UNSAFE: Code should ensure that self is uninitialised
	unsafe fn initialise(&mut self, size: usize) {
		::core::ptr::write(self, Block {
			state: BlockState::Free,
			size: size,
			});
		::core::ptr::write(self.tail(), BlockTail {
			head_ptr: self,
			});
	}

	fn tail(&mut self) -> &mut BlockTail {
		// SAFE: Mutably borrows self (which is eventually mut borrow of state)
		unsafe {
			&mut *((self as *const Block as usize + self.size - size_of::<BlockTail>()) as *mut BlockTail)
		}
	}
	// Safe, unlike prev, because it doesn't deref
	fn next(&self) -> *mut Block {
		(self as *const Block as usize + self.size) as *mut Block
	}
	/// UNSAFE: Must ensure that 'self' isn't the first
	unsafe fn prev(&self) -> *mut Block {
		let pt = (self as *const Block as usize - size_of::<BlockTail>()) as *mut BlockTail;
		(*pt).head_ptr
	}

	/// UNSAFE: Calls self.prev(), so has same caveats
	unsafe fn try_merge_left(&mut self) {
		let prev = &mut *self.prev();
		if let BlockState::Free = prev.state
		{
			prev.size += self.size;
			self.tail().head_ptr = prev;
		}
	}

	fn self_free(&mut self) -> Option<&mut Self> {
		if let BlockState::Free = self.state {
			Some(self)
		}
		else {
			None
		}
	}

	fn get_data_ofs(&self, align: usize) -> usize {
		let myaddr = self as *const _ as usize;
		assert_eq!(myaddr % align_of::<Block>(), 0);
		let alignment_error = (myaddr + size_of::<Block>()) % align;
		// TODO: Asserts that the error is zero, because otherwise going from the data pointer to
		// metadata pointer is very hard
		assert_eq!(alignment_error, 0);
		let padding = if alignment_error > 0 { align - alignment_error } else { 0 };
		size_of::<Block>() + padding
	}

	fn capacity(&self, align: usize) -> usize {
		self.size - self.get_data_ofs(align) - size_of::<BlockTail>()
	}
	fn allocate(&mut self, size: usize, align: usize) -> *mut () {
		let dataofs = self.get_data_ofs(align);
		assert!(dataofs == size_of::<Block>());
		//kernel_log!("Block::allocate(self={:p}, size={:#x}, align={}) cap = {:#x}", self, size, align, self.capacity(align));
		assert!(size <= self.capacity(align));

		if self.capacity(align) - size > MIN_BLOCK_SIZE
		{
			let new_self_size = (size_of::<Block>() + size + size_of::<BlockTail>() + BLOCK_ALIGN-1) & !(BLOCK_ALIGN-1);
			let new_other_size = self.size - new_self_size;
			// SAFE: Unique access, new block is valid (part of old block)
			unsafe {
				self.initialise(new_self_size);
				let next = &mut *self.next();
				next.initialise(new_other_size);
			}
			assert!(size <= self.capacity(align));
			//kernel_log!("- resized to cap = {:#x}", self.capacity(align));
		}
		//kernel_log!("- {}/{} bytes used", size, self.capacity(align));
		self.state = BlockState::Used(size);
		(self as *mut Block as usize + dataofs) as *mut ()
	}
}
