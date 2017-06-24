//
//
//
#![crate_type="rlib"]
#![crate_name="alloc_system"]
#![allocator]
#![feature(allocator)]
#![feature(lang_items)]	// Allow definition of lang_items
#![feature(const_fn)]
#![feature(unique)]
#![feature(box_syntax)]
#![feature(optin_builtin_traits)]	// For !Send
#![feature(unboxed_closures)]
#![no_std]

#[macro_use]
extern crate syscalls;
#[macro_use]
extern crate macros;

extern crate std_sync as sync;

mod std {
	pub use core::fmt;
}

mod heap;


pub fn oom() {
	panic!("Out of memory");
}

#[no_mangle]
pub extern "C" fn __rust_allocate(size: usize, align: usize) -> *mut u8
{
	heap::allocate(size, align)
}
#[no_mangle]
pub extern "C" fn __rust_allocate_zeroed(size: usize, align: usize) -> *mut u8
{
	let ptr = heap::allocate(size, align);
	// SAFE: Allocated pointer
	unsafe { ::core::ptr::write_bytes(ptr, 0, size); }
	ptr
}
#[no_mangle]
pub unsafe extern "C" fn __rust_deallocate(ptr: *mut u8, old_size: usize, align: usize)
{
	heap::deallocate(ptr, old_size, align)
}
#[no_mangle]
pub unsafe extern "C" fn __rust_reallocate(ptr: *mut u8, old_size: usize, size: usize, align: usize) -> *mut u8
{
	heap::reallocate(ptr, old_size, align, size)
}
#[no_mangle]
pub unsafe extern "C" fn __rust_reallocate_inplace(ptr: *mut u8, old_size: usize, size: usize, align: usize) -> usize
{
	heap::reallocate_inplace(ptr, old_size, align, size)
}
#[no_mangle]
pub extern "C" fn __rust_usable_size(size: usize, align: usize) -> usize
{
	heap::get_usable_size(size, align)
}
