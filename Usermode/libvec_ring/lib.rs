//! Vector backed ring buffer
#![feature(raw_vec_internals)]

extern crate alloc;
use alloc::collections::vec_deque::{self, VecDeque};

pub struct VecRing<T> {
	inner: VecDeque<T>,
}

impl<T> VecRing<T> {
	pub fn new() -> VecRing<T> {
		VecRing {
			inner: VecDeque::new(),
		}
	}
	pub fn with_capacity(cap: usize) -> VecRing<T> {
		VecRing {
			inner: VecDeque::with_capacity(cap),
		}
	}

	pub fn len(&self) -> usize {
		self.inner.len()
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
		if self.inner.capacity() == 0 {
			// Just drop, no capacity
			false
		}
		else if self.inner.len() < self.inner.capacity() {
			self.inner.push_back(v);
			true
		}
		else {
			// Overwrite first item and shift base up
			self.inner.pop_front();
			self.inner.push_back(v);
			false
		}
	}
	pub fn push_front(&mut self, v: T) -> bool {
		if self.inner.capacity() == 0 {
			// Just drop, no capacity
			false
		}
		else if self.inner.len() < self.inner.capacity() {
			self.inner.push_front(v);
			true
		}
		else {
			self.inner.pop_back();
			self.inner.push_front(v);
			false
		}
	}

	pub fn iter(&self) -> Iter<T> {
		Iter {
			inner_it: self.inner.iter(),
		}
	}
}

impl<T> ::std::ops::Index<usize> for VecRing<T> {
	type Output = T;
	fn index(&self, idx: usize) -> &T {
		&self.inner[idx]
	}
}
impl<T> ::std::ops::IndexMut<usize> for VecRing<T> {
	fn index_mut(&mut self, idx: usize) -> &mut T {
		&mut self.inner[idx]
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
	inner_it: vec_deque::Iter<'a, T>, 
}
impl<'a, T: 'a> Iterator for Iter<'a, T> {
	type Item = &'a T;
	fn next(&mut self) -> Option<&'a T> {
		self.inner_it.next()
	}
}

