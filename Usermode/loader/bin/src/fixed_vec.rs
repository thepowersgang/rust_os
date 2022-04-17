//!
//! Fixed-size vector (fixed to 16 entries)
//! 
use std::mem::MaybeUninit;

pub struct FixedVec<T> {
	size: usize,
	data: MaybeUninit<[T; 16]>,
}
impl<T> FixedVec<T> {
	pub fn new() -> FixedVec<T> {
		// SAFE: Won't be read until written to
		FixedVec { size: 0, data: MaybeUninit::uninit(), }
	}
	pub fn push(&mut self, v: T) -> Result<(),T> {
		if self.size == 16 {
			Err(v)
		}
		else {
			// SAFE: Writing to newly made-valid cell
			unsafe { ::std::ptr::write( (self.data.as_mut_ptr() as *mut T).offset(self.size as isize), v ) };
			self.size += 1;
			Ok( () )
		}
	}
}
impl<T> ::std::ops::Deref for FixedVec<T> {
	type Target = [T];
	fn deref(&self) -> &[T] {
		// SAFE: Initialised region
		unsafe { &(*self.data.as_ptr())[..self.size] }
	}
}
impl<T> ::std::ops::DerefMut for FixedVec<T> {
	fn deref_mut(&mut self) -> &mut [T] {
		// SAFE: Initialised region
		unsafe { &mut (*self.data.as_mut_ptr())[..self.size] }
	}
}