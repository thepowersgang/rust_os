//! 
//!
//!
use core::ptr::NonNull;
use core::alloc::{Layout,AllocRef,AllocError};

use alloc_system::ALLOCATOR as System;

#[no_mangle]
pub unsafe extern fn __rdl_alloc(size: usize, align: usize) -> *mut u8
{
	let layout = Layout::from_size_align_unchecked(size, align);
	match System.alloc(layout) {
		Ok(blk) => blk.as_ptr() as *mut u8,
		Err(AllocError) => {
			0 as *mut u8
		}
	}
}

#[alloc_error_handler]
pub fn rust_oom(_layout: Layout) -> ! {
	//System.oom()
	panic!("OOM");
}

#[no_mangle]
pub unsafe extern fn __rdl_dealloc(ptr: *mut u8,
								   size: usize,
								   align: usize) {
	System.dealloc(NonNull::new_unchecked(ptr as *mut _), Layout::from_size_align_unchecked(size, align))
}

#[no_mangle]
#[cfg(_false)]
pub unsafe extern fn __rdl_usable_size(layout: *const u8,
									   min: *mut usize,
									   max: *mut usize) {
	let pair = System.usable_size(&*(layout as *const Layout));
	*min = pair.0;
	*max = pair.1;
}

#[no_mangle]
pub unsafe extern fn __rdl_realloc(ptr: *mut u8, old_size: usize, old_align: usize, new_size: usize, ) -> *mut u8
{
	let old_layout = Layout::from_size_align_unchecked(old_size, old_align);
	let new_layout = Layout::from_size_align_unchecked(new_size, old_align);
	let rv = if old_size < new_size {
			System.grow(NonNull::new_unchecked(ptr as *mut _), old_layout, new_layout)
		}
		else {
			System.shrink(NonNull::new_unchecked(ptr as *mut _), old_layout, new_layout)
		};
	match rv {
		Ok(blk) => blk.as_ptr() as *mut u8,
		Err(AllocError) => {
			0 as *mut u8
		}
	}
}

#[no_mangle]
pub unsafe extern fn __rdl_alloc_zeroed(size: usize, align: usize,) -> *mut u8 {
	let layout = Layout::from_size_align_unchecked(size, align);
	match System.alloc_zeroed(layout) {
		Ok(blk) => {
			blk.as_ptr() as *mut u8
			},
		Err(AllocError) => {
			0 as *mut u8
		}
	}
}

