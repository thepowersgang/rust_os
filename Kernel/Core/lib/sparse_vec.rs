// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/sparse_vec.rs
// - Sparse vector type
use _common::*;
use core::ops;

/// Sparse vector type
///
/// A wrapper around Vec<Option<T>> for use as a resource pool
// Trailing usize is the number of populated elements
pub struct SparseVec<T>(Vec<Option<T>>,usize);

pub struct Element<'a,T: 'a>
{
	vec: &'a mut SparseVec<T>,
	idx: usize,
}

impl<T> SparseVec<T>
{
	pub fn new() -> SparseVec<T> {
		SparseVec(Vec::new(), 0)
	}

	pub fn insert(&mut self, data: T) -> usize {
		for (i,e) in (self.0).iter_mut().enumerate()
		{
			if e.is_none() {
				*e = Some(data);
				self.1 += 1;
				return i;
			}
		}
		self.0.push( Some(data) );
		self.1 += 1;
		self.0.len() - 1
	}
	pub fn find_free<'a>(&'a mut self) -> Option<Element<'a,T>> {
		None
	}
	pub fn push<'a>(&'a mut self, data: T) -> Element<'a,T> {
		self.0.push( Some(data) );
		self.1 += 1;
		Element { idx: self.0.len() - 1, vec: self, }
	}
	
	pub fn len(&self) -> usize { self.0.len() }
	pub fn count(&self) -> usize { self.1 }
}

impl<T> ops::Index<usize> for SparseVec<T>
{
	type Output = T;
	fn index(&self, idx: usize) -> &T {
		self.0[idx].as_ref().unwrap()
	}
}
impl<T> ops::IndexMut<usize> for SparseVec<T>
{
	fn index_mut(&mut self, idx: usize) -> &mut T {
		self.0[idx].as_mut().unwrap()
	}
}

impl<'a, T: 'a> Element<'a, T>
{
	pub fn get_index(&self) -> usize { self.idx }
	
	pub fn set(&mut self, v: T) {
		assert!( self.vec.0[self.idx].is_none() );
		self.vec.0[self.idx] = Some(v);
		self.vec.1 += 1;
	}
	pub fn clear(&mut self) {
		assert!( self.vec.0[self.idx].is_some() );
		self.vec.0[self.idx] = None;
		self.vec.1 -= 1;
	}
}
