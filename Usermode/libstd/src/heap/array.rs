//
//
//
use core::mem::{size_of,align_of};
use core::ptr::Unique;
use super::alloc::Allocation;

pub struct ArrayAlloc<T>
{
	base: Allocation<T>,
	size: usize,
}

impl<T> ArrayAlloc<T>
{
	pub fn new(size: usize) -> ArrayAlloc<T> {
		ArrayAlloc {
			// SAFE: Upper level code ensures size is correct
			base: unsafe { Allocation::new(size * size_of::<T>()).expect("ArrayAlloc::new") },
			size: size,
		}
	}
	
	pub fn resize(&mut self, newsize: usize) -> bool {
		// SAFE: This struct only exposes raw pointers, so any size is valid
		if unsafe { self.base.try_resize(newsize * size_of::<T>()) } {
			self.size = newsize;
			// Oh, good
			true
		}
		else {
			todo!("ArrayAlloc::expand({})", newsize);
		}
	}

	pub fn count(&self) -> usize {
		self.size
	}
	pub fn get_base(&self) -> *const T {
		*self.base
	}
	pub fn get_base_mut(&mut self) -> *mut T {
		*self.base
	}
	pub fn get_ptr(&self, idx: usize) -> *const T {
		// SAFE: Bounds checked
		unsafe {
			assert!(idx < self.size);
			self.base.offset( idx as isize )
		}
	}
	pub fn get_ptr_mut(&mut self, idx: usize) -> *mut T {
		// SAFE: Bounds checked
		unsafe {
			assert!(idx < self.size);
			self.base.offset( idx as isize )
		}
	}
}

