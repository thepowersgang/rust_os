//
//
//
use core::mem::{size_of,align_of};
use core::ptr::Unique;
use super::alloc::{exchange_malloc,exchange_free};

pub struct ArrayAlloc<T>
{
	base: Unique<T>,
	size: usize,
}

impl<T> ArrayAlloc<T>
{
	pub fn new(size: usize) -> ArrayAlloc<T> {
		ArrayAlloc {
			base: unsafe { Unique::new(exchange_malloc(size * size_of::<T>(), align_of::<T>()) as *mut T) },
			size: size,
		}
	}
	
	pub fn expand(&mut self, newsize: usize) -> bool {
		todo!("ArrayAlloc::expand({})", newsize);
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

