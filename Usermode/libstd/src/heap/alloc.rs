//
//
//
use prelude::v1::*;
use sync::Mutex;
use core::mem::{align_of,size_of};
use core::ptr::Unique;

#[cfg(arch__amd64)]
const HEAP_LIMITS: (usize,usize) = (0x1000_00000000, 0x7000_00000000);

static S_GLOBAL_HEAP: Mutex<AllocState> = Mutex::new(AllocState { start: 0 as *mut _, past_end: 0 as *mut _ } );

// Used by Box<T>
#[lang="exchange_malloc"]
pub unsafe fn exchange_malloc(size: usize, align: usize) -> *mut u8
{
	match S_GLOBAL_HEAP.lock().allocate(size, align)
	{
	Ok(x) => x as *mut u8,
	Err(_) => panic!("exchange_malloc({}, {}) out of memory", size, align),
	}
}
#[lang="exchange_free"]
pub unsafe fn exchange_free(ptr: *mut u8, _size: usize, align: usize)
{
	S_GLOBAL_HEAP.lock().deallocate(ptr as *mut (), /*size,*/ align)
}

pub struct Allocation<T>
{
	ptr: Unique<T>,
}

impl<T> Allocation<T>
{
	pub unsafe fn new(bytes: usize) -> Result<Allocation<T>, ()> {
		assert!(bytes == 0 || bytes >= size_of::<T>());
		S_GLOBAL_HEAP.lock().allocate(bytes, align_of::<T>()).map(|v| Allocation { ptr: Unique::new(v as *mut T) })
	}
	pub unsafe fn try_resize(&mut self, newbytes: usize) -> bool {
		let mut lh = S_GLOBAL_HEAP.lock();
		if *self.ptr == 1 as *mut T {
			self.ptr = Unique::new( lh.allocate(newbytes, align_of::<T>()).unwrap() as *mut T );
			true
		}
		else {
			lh.try_expand( *self.ptr as *mut (), newbytes, align_of::<T>() )
		}
	}
}
impl<T> ::core::ops::Deref for Allocation<T>
{
	type Target = *mut T;
	fn deref(&self) -> &*mut T {
		&*self.ptr
	}
}
impl<T> ::core::ops::Drop for Allocation<T>
{
	fn drop(&mut self) {
		unsafe {
			S_GLOBAL_HEAP.lock().deallocate(*self.ptr as *mut (), align_of::<T>());
		}
	}
}

struct AllocState
{
	start: *mut Block,
	past_end: *mut Block,
//	first_free: BlockPointer,
}
unsafe impl Send for AllocState {}

struct Block
{
	size: usize,
	state: BlockState,
}
#[derive(PartialEq)]
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
		if size == 0 {
			return Ok( 1 as *mut () );
		}
		if self.start == self.past_end {
			self.extend_reservation(size);

			let block = self.last_block();
			return Ok(block.allocate(size, align));
		}
		
		for block in self.free_blocks()
		{
			// TODO: Block split
			if block.capacity(align) >= size {
				return Ok(block.allocate(size, align));
			}
		}

		let current_extra = self.last_block().self_free().map(|blk| blk.capacity(align)).unwrap_or(0);
		self.extend_reservation(size - current_extra);

		let block = self.last_block();
		return Ok(block.allocate(size, align));
	}
	/// Returns 'true' if expanding succeeded
	pub unsafe fn try_expand(&mut self, ptr: *mut (), size: usize, align: usize) -> bool {
		if size == 0 {
			// TODO: Resize down to 0?
			true
		}
		else if ptr == 1 as *mut () {
			false
		}
		else {
			let bp = Block::ptr_from_ptr(ptr, align);
			if (*bp).capacity(align) > size {
				(*bp).state = BlockState::Used( size );
				true
			}
			else {
				let n = (*bp).next();
				if n == self.past_end {
					false
				}
				else if (*n).self_free().is_some() {
					todo!("AllocState::try_expand - Next is free");
				}
				else {
					false
				}
			}
		}
	}
	pub unsafe fn deallocate(&mut self, ptr: *mut (), align: usize) {
		if ptr == 1 as *mut () {
			return ;
		}
		else {
			let bp = Block::ptr_from_ptr(ptr, align);
			(*bp).state = BlockState::Free;
			let np = (*bp).next();
			if np == self.past_end {
				// Final block
			}
			else if let Some(next) = (*np).self_free() {
				todo!("AllocState::deallocate - Merge with next block");
			}
			else {
				// Next block isn't free, can't merge
			}
		}
	}

	fn last_block(&mut self) -> &mut Block {
		unsafe { &mut *(*self.past_end).prev() }
	}
	fn extend_reservation(&mut self, required_space: usize) {
		let npages = (required_space + size_of::<Block>() + size_of::<BlockTail>() + 0xFFF) >> 12;
		assert!(npages > 0);
		assert!(self.past_end != HEAP_LIMITS.1 as *mut Block);
		if self.start.is_null() {
			self.start = HEAP_LIMITS.0 as *mut Block;
			self.past_end = HEAP_LIMITS.0 as *mut Block;
		}

		let cb = unsafe {
			::syscalls::memory::allocate(self.past_end as usize, npages).expect("Heap allocation failure");
			(*self.past_end).initialise(npages << 12);
			&mut *self.past_end
			};
		self.past_end = cb.next();
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
impl<'a> ::std::iter::Iterator for FreeBlocks<'a>
{
	type Item = &'a mut Block;
	fn next(&mut self) -> Option<Self::Item> {
		while self.cur != self.state.past_end {
			let block = unsafe {
				let bp = self.cur;
				self.cur = (*self.cur).next();
				&mut *bp
				};
			if let BlockState::Free = block.state {
				return Some(block);
			}
		}
		None
	}
}

impl Block
{ 
	unsafe fn ptr_from_ptr(ptr: *mut (), /*size: usize,*/ align: usize) -> *mut Block {
		let bp_us = ptr as usize - size_of::<Block>();
		let bp = bp_us as *mut Block;
		//assert!( (*bp).state == BlockState::Used(size) );
		bp
	}

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
		unsafe {
			&mut *((self as *const Block as usize + self.size - size_of::<BlockTail>()) as *mut BlockTail)
		}
	}
	fn next(&self) -> *mut Block {
		(self as *const Block as usize + self.size) as *mut Block
	}
	/// UNSAFE: Must ensure that 'self' isn't the first
	unsafe fn prev(&self) -> *mut Block {
		let pt = (self as *const Block as usize - size_of::<BlockTail>()) as *mut BlockTail;
		(*pt).head_ptr
	}

	fn self_free(&self) -> Option<&Self> {
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
		kernel_log!("size = {}, cap = {}", size, self.capacity(align));
		assert!(size <= self.capacity(align));

		self.state = BlockState::Used(size);
		(self as *mut Block as usize + dataofs) as *mut ()
	}
}

