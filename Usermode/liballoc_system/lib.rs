//
//
//
#![crate_type="rlib"]
#![crate_name="alloc_system"]
#![feature(lang_items)]	// Allow definition of lang_items
#![feature(box_syntax)]
//#![feature(optin_builtin_traits)]	// For !Send
#![feature(unboxed_closures)]
#![feature(allocator_api)]
#![no_std]

use core::alloc::{self,Layout,AllocError};
use core::ptr::NonNull;

#[macro_use]
extern crate syscalls;

extern crate std_sync as sync;

mod std {
	pub use core::fmt;
}

mod heap;


pub fn oom() {
	panic!("Out of memory");
}


pub struct Allocator;
pub const ALLOCATOR: Allocator = Allocator;

unsafe impl alloc::Allocator for Allocator
{
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError>
	{
		match heap::S_GLOBAL_HEAP.lock().allocate(layout.size(), layout.align())
		{
		Ok(rv) => {
			// SAFE: Non-zero pointer
			Ok(unsafe { NonNull::new_unchecked(::core::slice::from_raw_parts_mut(rv as *mut u8, usable_size(&layout))) })
			},
		Err( () ) => {
			Err(AllocError)
			}
		}
	}
	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout)
	{
		heap::S_GLOBAL_HEAP.lock().deallocate(ptr.as_ptr() as *mut (), /*layout.size(),*/ layout.align());
	}

	unsafe fn grow(&self, ptr: NonNull<u8>, layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError>
	{
		let mut lh = heap::S_GLOBAL_HEAP.lock();
		assert!(layout.size() <= new_layout.size());
		assert!(layout.align() == new_layout.align());
		match lh.try_expand(ptr.as_ptr() as *mut (), new_layout.size(), layout.align())
		{
		Ok( () ) => {
			let true_new_size = heap::get_usable_size(new_layout.size(), layout.align()).0;
			Ok( NonNull::new_unchecked(::core::slice::from_raw_parts_mut(ptr.as_ptr(), true_new_size)) )
			},
		Err( () ) => {
			// Can't just expand the current alloc, so need to allocate a new buffer and copy
			// - Allocate
			let new_alloc = lh.allocate(new_layout.size(), layout.align()).map_err(|_| AllocError)? as *mut u8;
			// - Copy
			::core::ptr::copy_nonoverlapping(ptr.as_ptr(), new_alloc, layout.size());
			// - Free the original
			lh.deallocate(ptr.as_ptr() as *mut (), layout.align());
			// Return
			let true_new_size = heap::get_usable_size(new_layout.size(), layout.align()).0;
			Ok( NonNull::new_unchecked(::core::slice::from_raw_parts_mut(new_alloc, true_new_size)) )
			}
		}
	}
	//unsafe fn shrink(&mut self, ptr: NonNull<u8>, layout: Layout, new_size: usize) -> Result<MemoryBlock, AllocError>
}

fn usable_size(layout: &Layout) -> usize
{
	heap::get_usable_size(layout.size(), layout.align()).0
}
