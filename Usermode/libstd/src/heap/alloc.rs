//
//
//
use prelude::v1::*;
use sync::Mutex;

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
pub unsafe fn exchange_free(ptr: *mut u8, size: usize, align: usize)
{
	S_GLOBAL_HEAP.lock().deallocate(ptr as *mut (), size, align)
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
enum BlockState
{
	Free,
	Used(usize),
}
struct BlockTail
{
	head_ptr: *const Block,
}

impl AllocState
{
	pub fn allocate(&mut self, size: usize, align: usize) -> Result<*mut (), ()>
	{
		if self.start == self.past_end {
			todo!("Begin allocating memory for user heap");
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

	pub fn deallocate(&mut self, ptr: *mut (), size: usize, align: usize) {
	}

	fn last_block(&mut self) -> &mut Block {
		todo!("last_block");
	}
	fn extend_reservation(&mut self, required_space: usize) {
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
				self.cur = (self.cur as usize + (*self.cur).size) as *mut Block;
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
	fn self_free(&self) -> Option<&Self> {
		if let BlockState::Free = self.state {
			Some(self)
		}
		else {
			None
		}
	}

	fn get_data_ofs(&self, align: usize) -> usize {
		use core::mem::{align_of,size_of};
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
		use core::mem::size_of;
		self.size - self.get_data_ofs(align) - size_of::<BlockTail>()
	}
	fn allocate(&mut self, size: usize, align: usize) -> *mut () {
		todo!("Block::allocate");
	}
}

