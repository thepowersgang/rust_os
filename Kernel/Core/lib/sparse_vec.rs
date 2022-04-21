// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/sparse_vec.rs
//! Sparse vector type
//!
use crate::prelude::*;
use core::ops;

/// Sparse vector type
///
/// A wrapper around Vec<Option<T>> for use as a resource pool
pub struct SparseVec<T>
{
	data: Vec<Option<T>>,
	count: usize,
}

pub struct Element<'a,T: 'a>
{
	vec: &'a mut SparseVec<T>,
	idx: usize,
}
pub struct Iter<'a,T: 'a>
{
	vec: &'a SparseVec<T>,
	idx: usize,
}
pub struct IterMut<'a,T: 'a>
{
	vec: &'a mut SparseVec<T>,
	idx: usize,
}

impl<T> SparseVec<T>
{
	pub const fn new() -> SparseVec<T> {
		SparseVec { data: Vec::new(), count: 0 }
	}

	/// Insert an element anywhere within the vec and return the location
	pub fn insert(&mut self, data: T) -> usize {
		for (i,e) in self.data.iter_mut().enumerate()
		{
			if e.is_none() {
				*e = Some(data);
				self.count += 1;
				return i;
			}
		}
		self.data.push( Some(data) );
		self.count += 1;
		self.data.len() - 1
	}
	/// Remove the item at the specified location
	pub fn remove(&mut self, idx: usize) {
		if idx < self.data.len() && self.data[idx].is_some()
		{
			self.data[idx] = None;
			self.count -= 1;
		}
	}
	
	pub fn get(&self, idx: usize) -> Option<&T> {
		match self.data.get(idx) {
		Some(r) => r.as_ref(),
		None => None,
		}
	}
	
	//pub fn find_free<'a>(&'a mut self) -> Option<Element<'a,T>> {
	//	None
	//}
	/// Pushes an element onto the end of the list
	pub fn push<'a>(&'a mut self, data: T) -> Element<'a,T> {
		self.data.push( Some(data) );
		self.count += 1;
		Element { idx: self.data.len() - 1, vec: self, }
	}
	
	pub fn len(&self) -> usize { self.data.len() }
	pub fn count(&self) -> usize { self.count }
	
	pub fn iter<'a>(&'a self) -> Iter<'a, T> {
		Iter { vec: self, idx: 0 }
	}
	pub fn iter_mut<'a>(&'a mut self) -> IterMut<'a, T> {
		IterMut { vec: self, idx: 0 }
	}
}
impl<T> Default for SparseVec<T> {
	fn default() -> SparseVec<T> {
		SparseVec::new()
	}
}

impl<T> ops::Index<usize> for SparseVec<T>
{
	type Output = T;
	fn index(&self, idx: usize) -> &T {
		self.data[idx].as_ref().unwrap()
	}
}
impl<T> ops::IndexMut<usize> for SparseVec<T>
{
	fn index_mut(&mut self, idx: usize) -> &mut T {
		self.data[idx].as_mut().unwrap()
	}
}

impl<'a, T: 'a> Element<'a, T>
{
	pub fn get_index(&self) -> usize { self.idx }
	
	pub fn set(&mut self, v: T) {
		assert!( self.vec.data[self.idx].is_none() );
		self.vec.data[self.idx] = Some(v);
		self.vec.count += 1;
	}
	pub fn clear(&mut self) {
		assert!( self.vec.data[self.idx].is_some() );
		self.vec.data[self.idx] = None;
		self.vec.count -= 1;
	}
}

impl<'a, T: 'a> Iterator for Iter<'a, T>
{
	type Item = &'a T;
	
	fn next(&mut self) -> Option<&'a T> {
		while self.idx < self.vec.len()
		{
			self.idx += 1;
			match self.vec.data[self.idx-1]
			{
			Some(ref v) => return Some(v),
			None => {},
			}
		}
		None
	}
}

impl<'a, T: 'a> Iterator for IterMut<'a, T>
{
	type Item = &'a mut T;
	
	fn next(&mut self) -> Option<&'a mut T> {
		while self.idx < self.vec.len()
		{
			self.idx += 1;
			match self.vec.data[self.idx-1].as_mut()
			{
			// SAFE: While iterator exists, only one &mut to each element is returned
			Some(v) => return unsafe { Some( ::core::mem::transmute(v) ) },
			None => {},
			}
		}
		None
	}
}

