//! 
//!
//!
use core::ptr;

pub use alloc::allocator::{Layout,Alloc,AllocErr};

use alloc_system::ALLOCATOR as System;

#[no_mangle]
pub unsafe extern fn __rdl_alloc(size: usize,
								 align: usize,
								 err: *mut u8) -> *mut u8 {
	let layout = Layout::from_size_align_unchecked(size, align);
	match System.alloc(layout) {
		Ok(p) => p,
		Err(e) => {
			ptr::write(err as *mut AllocErr, e);
			0 as *mut u8
		}
	}
}

#[no_mangle]
pub unsafe extern fn __rdl_oom(err: *const u8) -> ! {
	System.oom((*(err as *const AllocErr)).clone())
}

#[no_mangle]
pub unsafe extern fn __rdl_dealloc(ptr: *mut u8,
								   size: usize,
								   align: usize) {
	System.dealloc(ptr, Layout::from_size_align_unchecked(size, align))
}

#[no_mangle]
pub unsafe extern fn __rdl_usable_size(layout: *const u8,
									   min: *mut usize,
									   max: *mut usize) {
	let pair = System.usable_size(&*(layout as *const Layout));
	*min = pair.0;
	*max = pair.1;
}

#[no_mangle]
pub unsafe extern fn __rdl_realloc(ptr: *mut u8,
								   old_size: usize,
								   old_align: usize,
								   new_size: usize,
								   new_align: usize,
								   err: *mut u8) -> *mut u8 {
	let old_layout = Layout::from_size_align_unchecked(old_size, old_align);
	let new_layout = Layout::from_size_align_unchecked(new_size, new_align);
	match System.realloc(ptr, old_layout, new_layout) {
		Ok(p) => p,
		Err(e) => {
			ptr::write(err as *mut AllocErr, e);
			0 as *mut u8
		}
	}
}

#[no_mangle]
pub unsafe extern fn __rdl_alloc_zeroed(size: usize,
										align: usize,
										err: *mut u8) -> *mut u8 {
	let layout = Layout::from_size_align_unchecked(size, align);
	match System.alloc_zeroed(layout) {
		Ok(p) => p,
		Err(e) => {
			ptr::write(err as *mut AllocErr, e);
			0 as *mut u8
		}
	}
}

#[no_mangle]
pub unsafe extern fn __rdl_alloc_excess(size: usize,
										align: usize,
										excess: *mut usize,
										err: *mut u8) -> *mut u8 {
	let layout = Layout::from_size_align_unchecked(size, align);
	match System.alloc_excess(layout) {
		Ok(p) => {
			*excess = p.1;
			p.0
		}
		Err(e) => {
			ptr::write(err as *mut AllocErr, e);
			0 as *mut u8
		}
	}
}

#[no_mangle]
pub unsafe extern fn __rdl_realloc_excess(ptr: *mut u8,
										  old_size: usize,
										  old_align: usize,
										  new_size: usize,
										  new_align: usize,
										  excess: *mut usize,
										  err: *mut u8) -> *mut u8 {
	let old_layout = Layout::from_size_align_unchecked(old_size, old_align);
	let new_layout = Layout::from_size_align_unchecked(new_size, new_align);
	match System.realloc_excess(ptr, old_layout, new_layout) {
		Ok(p) => {
			*excess = p.1;
			p.0
		}
		Err(e) => {
			ptr::write(err as *mut AllocErr, e);
			0 as *mut u8
		}
	}
}

#[no_mangle]
pub unsafe extern fn __rdl_grow_in_place(ptr: *mut u8,
										 old_size: usize,
										 old_align: usize,
										 new_size: usize,
										 new_align: usize) -> u8 {
	let old_layout = Layout::from_size_align_unchecked(old_size, old_align);
	let new_layout = Layout::from_size_align_unchecked(new_size, new_align);
	match System.grow_in_place(ptr, old_layout, new_layout) {
		Ok(()) => 1,
		Err(_) => 0,
	}
}

#[no_mangle]
pub unsafe extern fn __rdl_shrink_in_place(ptr: *mut u8,
										   old_size: usize,
										   old_align: usize,
										   new_size: usize,
										   new_align: usize) -> u8 {
	let old_layout = Layout::from_size_align_unchecked(old_size, old_align);
	let new_layout = Layout::from_size_align_unchecked(new_size, new_align);
	match System.shrink_in_place(ptr, old_layout, new_layout) {
		Ok(()) => 1,
		Err(_) => 0,
	}
}

