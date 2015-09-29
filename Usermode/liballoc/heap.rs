//
//
//
use sync::Mutex;
use core::mem::{align_of,size_of};
use core::ptr::Unique;

#[cfg(arch="amd64")]
const HEAP_LIMITS: (usize,usize) = (0x1000_0000_0000, 0x7000_0000_0000);
#[cfg(arch="armv7")]
const HEAP_LIMITS: (usize,usize) = (0x1000_0000, 0x7000_0000);

pub const EMPTY: *mut u8 = 1 as *mut u8;

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


pub fn alloc_typed<T>(value: T) -> *mut T
{
	match S_GLOBAL_HEAP.lock().allocate(size_of::<T>(), align_of::<T>())
	{
	Ok(ptr) => {
		let ptr = ptr as *mut T;
		// SAFE: Pointer is valid and uninitialised
		unsafe { ::core::ptr::write(ptr, value); }
		ptr
		},
	Err(_e) => todo!("OOM in alloc"),
	}
}
pub fn allocate(size: usize, align: usize) -> *mut u8
{
	match S_GLOBAL_HEAP.lock().allocate(size, align)
	{
	Ok(x) => x as *mut u8,
	Err(_) => panic!("alloc_raw({}, {}) out of memory", size, align),
	}
}
pub unsafe fn deallocate(ptr: *mut u8, _size: usize, align: usize)
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
	pub unsafe fn from_raw(ptr: *mut T) -> Allocation<T> {
		Allocation { ptr: Unique::new(ptr) }
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
		// SAFE: Pointer and size are valid
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
			return Ok( EMPTY as *mut () );
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
			// Nothing needs to be done, as the allocation was empty
		}
		else {
			let bp = Block::ptr_from_ptr(ptr, align);
			(*bp).state = BlockState::Free;
			let np = (*bp).next();
			if np == self.past_end {
				// Final block
			}
			else if let Some(next) = (*np).self_free() {
				let size = next.size;
				let foot = next.tail();
				(*bp).size += size;
				assert_eq!((*bp).tail() as *mut BlockTail, foot as *mut _);
				*foot = BlockTail { head_ptr: bp, };
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
		let npages = (required_space + size_of::<Block>() + size_of::<BlockTail>() + 0xFFF) >> 12;
		assert!(npages > 0);
		assert!(self.past_end != HEAP_LIMITS.1 as *mut Block);
		assert!(self.past_end as usize + (npages << 12) <= HEAP_LIMITS.1);	// TODO: This isn't an assert conditon, it's an OOM
		if self.start.is_null() {
			self.start = HEAP_LIMITS.0 as *mut Block;
			self.past_end = HEAP_LIMITS.0 as *mut Block;
		}

		// SAFE: Allocates only in controlled region.
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
		kernel_log!("size = {}, cap = {}", size, self.capacity(align));
		assert!(size <= self.capacity(align));

		self.state = BlockState::Used(size);
		(self as *mut Block as usize + dataofs) as *mut ()
	}
}

