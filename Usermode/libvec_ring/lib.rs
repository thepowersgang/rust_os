
extern crate alloc;

pub struct VecRing<T> {
	buf: alloc::raw_vec::RawVec<T>,
	base_pos: usize,
	len: usize,
}

impl<T> VecRing<T> {
	pub fn new() -> VecRing<T> {
		VecRing {
			buf: alloc::raw_vec::RawVec::new(),
			base_pos: 0,
			len: 0,
		}
	}
	pub fn with_capacity(cap: usize) -> VecRing<T> {
		VecRing {
			buf: alloc::raw_vec::RawVec::with_capacity(cap),
			base_pos: 0,
			len: 0,
		}
	}

	pub fn len(&self) -> usize {
		self.len
	}

	fn inner_ptr(&self, ofs: usize) -> *mut T {
		assert!(ofs < self.buf.cap());
		// SAFE: Returns raw pointer within bounds
		unsafe {
			self.buf.ptr().offset(ofs as isize)
		}
	}
	fn ptr(&self, idx: usize) -> *mut T {
		self.inner_ptr( (idx + self.base_pos) % self.buf.cap() )
	}

	/*
	/// Alter the maximum size of the ring buffer.
	pub fn resize(&mut self, new_cap: usize) {
		let n_tail = ::std::cmp::min(self.buf.cap() - self.base_pos, self.len());
		let n_head = self.len - n_tail;
	
		if self.buf.cap() == new_cap {
			// Welp, nothing to do
		}
		else if self.buf.cap() > new_cap {
			// Resize down.
			if self.base_pos >= new_cap {
				// Need to move early items back into range
				// (new_cap - n_tail --- new_cap) = (old_cap - n_tail --- old_cap)
				let src_base = old_cap - n_tail;
				if n_tail > new_cap {
					// Truncate tail items
				}
				else {
					let dst_base = new_cap - n_tail;
					for i in 0 .. n_tail
					{
						let src = self.inner_ptr(src_base + i);
						let dst = self.inner_ptr(dst_base + i);
						if dst_base + i < n_head {
							*dst = ::std::ptr::read(src);
						}
						else {
							::std::ptr::write(dst, ::std::ptr::read(src));
						}
					}
				}
			}
			else if self.base_pos + self.len > new_cap {
				// New capacity caused a wrap (old might have)
			}
			else {
				// base + len <= new_cap, no change needed
			}
			self.buf.reserve(0, new_cap);
		}
		else {
			// Resize up.
			self.buf.reserve(0, new_cap);
			// If the valid region crosses the end of the buffer
			if n_head > 0
			{
				// - Move tail items to the end of the list and update
				for i in (0 .. n_tail)
				{
					let src = self.innner_ptr(old_cap - 1 - i);
					let dst = self.innner_ptr(self.buf.cap() - 1 - i);
					::core::ptr::write(dst, ::core::ptr::read(src));
				}
			}
		}
	}
	*/

	pub fn push_back(&mut self, v: T) -> bool {
		if self.len < self.buf.cap() {
			// Write to an unused slot
			self.len += 1;
			// SAFE: Memory used is empty
			unsafe {
				::std::ptr::write(&mut self[0], v);
			}
			true
		}
		else {
			// Overwrite first item and shift base up
			self[0] = v;
			if self.base_pos == self.buf.cap() - 1 {
				self.base_pos = 0;
			}
			else {
				self.base_pos += 1;
			}
			false
		}
	}
	pub fn push_front(&mut self, v: T) -> bool {
		if self.buf.cap() == 0 {
			// Just drop, no capacity
			false
		}
		else if self.len < self.buf.cap() {
			// Write to an unused slot
			let pos = self.len;
			self.len += 1;
			// SAFE: Memory used is empty
			unsafe {
				::std::ptr::write(&mut self[pos], v);
			}

			true
		}
		else {
			// Overwrite last item and shift base up
			let pos = self.len - 1;
			self[pos] = v;

			if self.base_pos == 0 {
				self.base_pos = self.buf.cap() - 1;
			}
			else {
				self.base_pos -= 1;
			}

			false
		}
	}

	pub fn iter(&self) -> Iter<T> {
		Iter {
			ring: self,
			idx: 0,
		}
	}
}

impl<T> ::std::ops::Index<usize> for VecRing<T> {
	type Output = T;
	fn index(&self, idx: usize) -> &T {
		assert!(idx < self.len());
		// SAFE: Range-checked pointer into valid region
		unsafe { &*self.ptr(idx) }
	}
}
impl<T> ::std::ops::IndexMut<usize> for VecRing<T> {
	fn index_mut(&mut self, idx: usize) -> &mut T {
		assert!(idx < self.len());
		// SAFE: Range-checked pointer into valid region
		unsafe { &mut *self.ptr(idx) }
	}
}


impl<'a, T: 'a> ::std::iter::IntoIterator for &'a VecRing<T> {
	type IntoIter = Iter<'a, T>;
	type Item = &'a T;
	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

pub struct Iter<'a, T: 'a> {
	ring: &'a VecRing<T>,
	idx: usize,
}
impl<'a, T: 'a> Iterator for Iter<'a, T> {
	type Item = &'a T;
	fn next(&mut self) -> Option<&'a T> {
		None
	}
}

