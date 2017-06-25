// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/vec_map.rs
//! Sorted vector backed Key-Value map.
//use prelude::*;
//use core::{ops,fmt};
use memory::heap::ArrayAlloc;

pub struct VecDeque<T>
{
	data: ArrayAlloc<T>,
	start: usize,
	size: usize,
}
impl<T> Default for VecDeque<T>
{
	fn default() -> Self {
		Self::new()
	}
}

impl<T> VecDeque<T>
{
	pub const fn new() -> VecDeque<T>
	{
		VecDeque {
			data: ArrayAlloc::empty(),
			start: 0,
			size: 0,
			}
	}

	fn ensure_free_slot(&mut self) {
		if self.size == self.data.count() {
			self.data.expand(self.size + 1);
			if self.start > 0 {
				// Move from `start` .. `size - start` forward by `new_size - size`
				let delta = self.data.count() - self.size;
				for src_idx in (self.start .. self.size - self.start).rev()
				{
					let dst_idx = src_idx + delta;
					// SAFE: In-bounds access, write to unused region from soon-to-be invalid region
					unsafe {
						::core::ptr::write( self.data.get_ptr_mut(dst_idx), ::core::ptr::read(self.data.get_ptr(src_idx)) );
					}
				}

				self.start += delta;
			}
		}

		assert!(self.size < self.data.count());
	}

	pub fn is_empty(&self) -> bool {
		self.size == 0
	}

	pub fn push_back(&mut self, v: T) {
		self.ensure_free_slot();
		assert!(self.size < self.data.count());
		let idx = (self.start + self.size) % self.data.count();
		// SAFE: Write is to valid (and unused) memory
		unsafe { ::core::ptr::write(self.data.get_ptr_mut(idx), v); }

		self.size += 1;
		assert!(self.size <= self.data.count());
	}

	pub fn pop_back(&mut self) -> Option<T> {
		if self.size > 0 {
			let idx = (self.start + self.size) % self.data.count();
			// SAFE: Reads from valid and soon forgotten memory
			let rv = unsafe { ::core::ptr::read( self.data.get_ptr(idx) ) };
			self.size -= 1;
			Some(rv)
		}
		else {
			None
		}
	}

	pub fn push_front(&mut self, v: T) {
		self.ensure_free_slot();
		assert!(self.size < self.data.count());

		// SAFE: Writes to valid and unused memory
		unsafe { ::core::ptr::write(self.data.get_ptr_mut(self.start), v); }

		if self.start == 0 {
			self.start = self.data.count() - 1;
		}
		else {
			self.start -= 1;
		}
		self.size += 1;
		assert!(self.start < self.data.count());
		assert!(self.size <= self.data.count());
	}

	pub fn pop_front(&mut self) -> Option<T> {
		if self.size > 0 {
			// SAFE: Read from valid & initialised memory, pointer advanced after read
			let rv = unsafe { ::core::ptr::read( self.data.get_ptr(self.start) ) };
			self.start += 1;
			if self.start == self.data.count() {
				self.start = 0;
			}
			self.size -= 1;
			Some(rv)
		}
		else {
			None
		}
	}
}

