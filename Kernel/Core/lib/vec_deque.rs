// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/vec_deque.rs
//! Dynamic array backed dequeue
use crate::memory::heap::ArrayAlloc;

pub struct VecDeque<T>
{
	data: ArrayAlloc<T>,
	ofs: usize,
	len: usize,
}

impl<T> VecDeque<T>
{
	pub const fn new_const() -> VecDeque<T> {
		VecDeque {
			data: ArrayAlloc::empty(),
			ofs: 0,
			len: 0,
			}
	}

	fn reserve_cap(&mut self, size: usize) {
		let usize_bits: u32 = (::core::mem::size_of::<usize>() * 8) as u32;
		let newcap = crate::lib::num::round_up(size, 1 << (usize_bits - size.leading_zeros()));
		if newcap > self.data.count()
		{
			let orig_cap = self.data.count();
			if self.data.expand(newcap)
			{
				// Copy any entries that were in the front of the list
				let n_ents_before_end = orig_cap - self.ofs;
				let space_before_end = self.data.count() - self.ofs;
				if n_ents_before_end < self.len
				{
					let n_ents_to_move = self.len - n_ents_before_end;
					// Move this many entries from the start of the allocation to the end
					if space_before_end < self.len {
						// Insufficient space in the newly allocated space to fit all of the tail, partial copy
						let to_tail_count = space_before_end - orig_cap;
						let shift_back_count = self.len - space_before_end;
						// SAFE: Meh
						unsafe {
							::core::ptr::copy_nonoverlapping(self.data.get_ptr(0), self.data.get_ptr_mut(orig_cap), to_tail_count);
							::core::ptr::copy(self.data.get_ptr(to_tail_count), self.data.get_ptr_mut(0), shift_back_count);
						}
					}
					else {
						// Contigious copy
						// SAFE: Meh.
						unsafe {
							::core::ptr::copy_nonoverlapping(self.data.get_ptr(0), self.data.get_ptr_mut(orig_cap), n_ents_to_move);
						}
					}
				}
			}
			else
			{
				// Allocate a new version
				let mut new_alloc = ArrayAlloc::new(newcap);
				if self.len > 0
				{
					if self.data.count() - self.ofs < self.len {
						// Data is contigious
						// SAFE: Copying valid data
						unsafe {
							::core::ptr::copy(self.data.get_ptr(self.ofs), new_alloc.get_ptr_mut(0), self.len);
						}
					}
					else {
						// Data is _not_ contigious
						let seg1_len = self.data.count() - self.ofs;
						let seg2_len = self.len - seg1_len;
						// SAFE: Copying valid data
						unsafe {
							::core::ptr::copy(self.data.get_ptr(self.ofs), new_alloc.get_ptr_mut(0), seg1_len);
							::core::ptr::copy(self.data.get_ptr(0), new_alloc.get_ptr_mut(seg1_len), seg2_len);
						}
					}
				}
				// New allocation: Offset is now zero
				self.ofs = 0;
				//log_debug!("self.data={:?}, new_alloc = {:?}", self.data, new_alloc);
				self.data = new_alloc;
			}
		}
	}
	pub fn push_back(&mut self, v: T) {
		let new_len = self.len + 1;
		self.reserve_cap(new_len);
		let pos = (self.ofs + self.len) % self.data.count();
		// SAFE: Correct write
		unsafe {
			::core::ptr::write(self.data.get_ptr_mut(pos), v);
		}
		self.len += 1;
	}
	pub fn pop_front(&mut self) -> Option<T> {
		if self.len == 0 {
			None
		}
		else {
			let pos = self.ofs;
			self.len -= 1;
			self.ofs = (self.ofs + 1) % self.data.count();
			// SAFE: Correct read
			unsafe {
				Some( ::core::ptr::read(self.data.get_ptr(pos)) )
			}
		}
	}
}

