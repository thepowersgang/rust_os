//
//
//
#![crate_type="rlib"]
#![crate_name="alloc_system"]
//#![feature(lang_items)]	// Allow definition of lang_items
//#![feature(optin_builtin_traits)] // For !Send
#![feature(unboxed_closures)]
#![feature(allocator_api)]
#![no_std]

use core::alloc::{self,Layout,AllocError};

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
#[global_allocator]
pub static ALLOCATOR: Allocator = Allocator;

unsafe impl ::alloc::GlobalAlloc for Allocator
{
	unsafe fn alloc(&self, layout: Layout) -> *mut u8
	{
		match heap::S_GLOBAL_HEAP.lock().allocate(layout.size(), layout.align())
		{
		Ok(rv) => {
			rv as *mut u8
			},
		Err( () ) => {
			::core::ptr::null_mut()
			}
		}
	}
	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout)
	{
		heap::S_GLOBAL_HEAP.lock().deallocate(ptr as *mut (), /*layout.size(),*/ layout.align());
	}

	

	unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8
	{
		let mut lh = heap::S_GLOBAL_HEAP.lock();
		if new_size <= layout.size() {
			return ptr;
		}
		match lh.try_expand(ptr as *mut (), new_size, layout.align())
		{
		Ok( () ) => {
			//let true_new_size = heap::get_usable_size(new_size, layout.align()).0;
			ptr
			},
		Err( () ) => {
			// Can't just expand the current alloc, so need to allocate a new buffer and copy
			// - Allocate
			let new_alloc = match lh.allocate(new_size, layout.align()).map_err(|_| AllocError) {
				Ok(v) => v as *mut u8,
				Err(_) => return ::core::ptr::null_mut(),
			};
			// - Copy
			::core::ptr::copy_nonoverlapping(ptr, new_alloc, layout.size());
			// - Free the original
			lh.deallocate(ptr as *mut (), layout.align());
			// Return
			//let true_new_size = heap::get_usable_size(new_size, layout.align()).0;
			new_alloc
			}
		}
	}
}
