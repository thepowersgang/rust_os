//
//
//
#![crate_type="rlib"]
#![crate_name="alloc_system"]
#![feature(lang_items)]	// Allow definition of lang_items
#![feature(const_fn)]
#![feature(box_syntax)]
#![feature(optin_builtin_traits)]	// For !Send
#![feature(unboxed_closures)]
#![feature(allocator_api)]
#![no_std]

use core::alloc::{self,Layout,AllocErr};
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
pub const ALLOCATOR: &Allocator = &Allocator;

unsafe impl ::core::alloc::AllocRef for &'static Allocator
{
	fn alloc(&mut self, layout: Layout) -> Result<NonNull<[u8]>, AllocErr>
	{
		let rv = heap::allocate(layout.size(), layout.align());
		if rv == ::core::ptr::null_mut()
		{
			Err(AllocErr)
		}
		else
		{
			// SAFE: Non-zero pointer
			Ok(unsafe { NonNull::new_unchecked(::core::slice::from_raw_parts_mut(rv as *mut u8, usable_size(&layout))) })
		}
	}
	unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout)
	{
		heap::deallocate(ptr.as_ptr() as *mut u8, layout.size(), layout.align())
	}

	//unsafe fn grow(&mut self, ptr: NonNull<u8>, layout: Layout, new_size: usize) -> Result<MemoryBlock, AllocErr>
	//unsafe fn shrink(&mut self, ptr: NonNull<u8>, layout: Layout, new_size: usize) -> Result<MemoryBlock, AllocErr>
}

fn usable_size(layout: &Layout) -> usize
{
	heap::get_usable_size(layout.size(), layout.align()).0
}
