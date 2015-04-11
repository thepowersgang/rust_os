// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/vec.rs
//! Dynamically growable vector type
use core::prelude::*;
use core::iter::{FromIterator,IntoIterator};
use core::ops;
use lib::collections::{MutableSeq};
use core::ptr::Unique;
use memory::heap::ArrayAlloc;

// TODO: Replace allocation with a boxed slice (or some other managed allocation)
// - Maybe a heap-provided "Array" type that is safe to alloc/free, but unsafe to access

/// Growable array of items
pub struct Vec<T>
{
	data: ArrayAlloc<T>,
	size: usize,
}

/// Owning iterator
pub struct MoveItems<T>
{
	data: ArrayAlloc<T>,
	count: usize,
	ofs: usize,
}

impl<T> Vec<T>
{
	//pub static EMPTY: Vec<T> = Vec { data: Unique(::memory::heap::ZERO_ALLOC), size: 0, capacity: 0 };
	
	/// Create a new, empty vector
	pub fn new() -> Vec<T>
	{
		Vec::with_capacity(0)
	}
	/// Create a vector with an initialised capacity
	pub fn with_capacity(size: usize) -> Vec<T>
	{
		Vec {
			data: ArrayAlloc::new(size),
			size: 0,
		}
	}
	/// Populate vector using a provided callback
	pub fn from_fn<Fcn>(length: usize, op: Fcn) -> Vec<T>
	where
		Fcn: Fn(usize) -> T
	{
		let mut ret = Vec::with_capacity(length);
		for i in (0 .. length) {
			ret.push( op(i) );
		}
		ret
	}

	/// Obtain a mutable pointer to an item within the vector
	fn get_mut_ptr(&mut self, index: usize) -> *mut T
	{
		assert!(index < self.size, "Vec<{}>::get_mut_ptr(): Index out of range, {} >= {}", type_name!(T), index, self.size);
		self.data.get_ptr_mut(index)
	}
	
	/// Move contents into an iterator (consuming self)
	pub fn into_iter(mut self) -> MoveItems<T>
	{
		let dataptr = ::core::mem::replace(&mut self.data, ArrayAlloc::new(0));
		let count = self.size;
		unsafe { ::core::mem::forget(self) };
		MoveItems {
			data: dataptr,
			ofs: 0,
			count: count,
		}
	}

	pub fn clear(&mut self)
	{
		unimplemented!();
	}
	
	fn reserve_cap(&mut self, size: usize)
	{
		let newcap = ::lib::num::round_up(size, 1 << (64-size.leading_zeros()));
		if newcap > self.data.count()
		{
			if self.data.expand(newcap)
			{
				// All good
			}
			else
			{
				let mut newdata = ArrayAlloc::new(newcap);
				unsafe {
					for i in (0 .. self.size) {
						let val = self.move_ent(i as usize);
						::core::ptr::write(newdata.get_ptr_mut(i), val);
					}
				}
				log_debug!("Vec<{}>::reserve_cap({}): newdata={:?}", type_name!(T), size, newdata);
				self.data = newdata;
			}
		}
	}
	pub fn reserve(&mut self, extras: usize) {
		let newcap = self.size + extras;
		self.reserve_cap(newcap);
	}
	
	pub fn slice_mut<'a>(&'a mut self) -> &'a mut [T]
	{
		unsafe { ::core::mem::transmute( ::core::raw::Slice { data: self.data.get_base_mut(), len: self.size } ) }
	}
	
	/// Move out of a slot in the vector, leaving unitialise memory in its place
	unsafe fn move_ent(&mut self, pos: usize) -> T
	{
		::core::ptr::read(self.data.get_ptr(pos))
	}

	/// Insert an item at the specified index (moving subsequent items up)	
	pub fn insert(&mut self, pos: usize, value: T)
	{
		// Expand by one element
		let ns = self.size + 1;
		self.reserve_cap(ns);
		self.size = ns;
		unsafe
		{
			// Move elements (pos .. len) to (pos+1 .. len+1)
			for i in (pos .. self.size).rev()
			{
				let src = self.data.get_ptr( i );
				let dst = self.data.get_ptr_mut( i+1 );
				::core::ptr::write(dst, ::core::ptr::read(src));
			}
			// Store new element
			::core::ptr::write( self.data.get_ptr_mut(pos), value );
		}
	}
	
	pub fn truncate(&mut self, newsize: usize)
	{
		if newsize >= self.size
		{
			unsafe
			{
				for i in (newsize .. self.size) {
					::core::mem::drop( ::core::ptr::read(self.get_mut_ptr(i) as *const T) );
				}
				self.size = newsize;
			}
		}
	}
}

#[macro_export]
macro_rules! vec
{
	($( $v:expr ),*) => ({
		let mut v = $crate::lib::Vec::new();
		v.reserve( _count!( $($v),* ) );
		$( v.push($v); )*
		v
		});
}

impl<T: Clone> Vec<T>
{
	pub fn resize(&mut self, new_len: usize, value: T)
	{
		if self.len() > new_len {
			self.truncate(new_len);
		}
		else {
			self.reserve_cap(new_len);
			for _ in self.size .. new_len {
				self.push(value.clone());
			}
		}
	}
}

impl<T> ::core::default::Default for Vec<T>
{
	fn default() -> Vec<T> { Vec::new() }
}

impl<T> ops::Deref for Vec<T>
{
	type Target = [T];
	fn deref(&self) -> &[T] {
		self.as_slice()
	}
}
impl<T> ops::DerefMut for Vec<T>
{
	fn deref_mut(&mut self) -> &mut [T] {
		self.slice_mut()
	}
}

impl<T:Clone> Vec<T>
{
	pub fn from_elem(size: usize, elem: T) -> Vec<T>
	{
		let mut ret = Vec::with_capacity(size);
		for _ in 0 .. size-1 {
			ret.push(elem.clone());
		}
		ret.push(elem);
		ret
	}
	
	pub fn push_all(&mut self, other: &[T])
	{
		self.reserve(other.len());
		for v in other.iter() {
			self.push(v.clone());
		}
	}
}

impl<T> ops::Drop for Vec<T>
{
	fn drop(&mut self)
	{
		log_debug!("Vec::<{}>::drop() - {:?} w/ {} ents", type_name!(T), self.data, self.size);
		unsafe {
			while self.size > 0 {
				self.size -= 1;
				let idx = self.size;
				let ptr = self.data.get_ptr(idx) as *const T;
				::core::mem::drop( ::core::ptr::read(ptr) );
			}
		}
	}
}

macro_rules! vec_index {
	($T:ident -> $rv:ty : $($idx:ty)*) => { $(
		impl<$T> ops::Index<$idx> for Vec<$T>
		{
			type Output = $rv;
			fn index<'a>(&'a self, index: $idx) -> &'a $rv
			{
				&self.as_slice()[index]
			}
		}
		impl<$T> ops::IndexMut<$idx> for Vec<$T>
		{
			fn index_mut<'a>(&'a mut self, index: $idx) -> &'a mut $rv
			{
				&mut self.slice_mut()[index]
			}
		}
		)* }
	}
vec_index!{ T -> T : usize }
vec_index!{ T -> [T] : ops::Range<usize> ops::RangeTo<usize> ops::RangeFrom<usize> ops::RangeFull }

impl<T> ::core::slice::AsSlice<T> for Vec<T>
{
	fn as_slice<'a>(&'a self) -> &'a [T]
	{
		let rawslice = ::core::raw::Slice { data: self.data.get_base() as *const T, len: self.size };
		unsafe { ::core::mem::transmute( rawslice ) }
	}
}

//impl<T: ::core::fmt::Show> ::core::fmt::Show for Vec<T>
//{
//	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::result::Result<(),::core::fmt::Error>
//	{
//		write!(f, "{}", self.as_slice())
//	}
//}

impl<T> MutableSeq<T> for Vec<T>
{
	fn push(&mut self, t: T)
	{
		let pos = self.size;
		self.reserve(1);
		self.size += 1;
		let ptr = self.get_mut_ptr(pos);
		//log_debug!("Vec.push {}", HexDump(&t));
		unsafe { ::core::ptr::write(ptr, t); }
	}
	fn pop(&mut self) -> Option<T>
	{
		if self.size == 0
		{
			None
		}
		else
		{
			self.size -= 1;
			let pos = self.size;
			Some( unsafe { self.move_ent(pos) } )
		}
	}
}

impl<T> FromIterator<T> for Vec<T>
{
	fn from_iter<IT>(src: IT) -> Vec<T>
	where
		IT: ::core::iter::IntoIterator<Item=T>
	{
		let iterator = src.into_iter();
		let mut ret = Vec::new();
		if let (_, Some(size)) = iterator.size_hint()
		{
			ret.reserve_cap(size);
		}
		for val in iterator
		{
			ret.push(val);
		}
		ret
	}
}

impl<'a, T> IntoIterator for &'a Vec<T>
{
	type IntoIter = ::core::slice::Iter<'a,T>;
	type Item = &'a T;
	
	fn into_iter(self) -> ::core::slice::Iter<'a, T> {
		self.iter()
	}
}

impl<'a, T> IntoIterator for &'a mut Vec<T>
{
	type IntoIter = ::core::slice::IterMut<'a,T>;
	type Item = &'a mut T;
	
	fn into_iter(self) -> ::core::slice::IterMut<'a, T> {
		self.iter_mut()
	}
}

impl<T> MoveItems<T>
{
	/// Pop an item from the iterator
	fn pop_item(&mut self) -> T
	{
		//log_debug!("MoveItems.pop_item() ofs={}, count={}, data={}", self.ofs, self.count, self.data);
		assert!(self.ofs < self.count);
		let v: T = unsafe {
			let ptr = self.data.get_ptr(self.ofs);
			::core::ptr::read(ptr as *const _)
			};
		//log_debug!("MoveItems.pop_item() v = {}", HexDump(&v));
		self.ofs += 1;
		v
	}
}

impl<T> Iterator for MoveItems<T>
{
	type Item = T;
	fn next(&mut self) -> Option<T>
	{
		if self.ofs == self.count
		{
			None
		}
		else
		{
			Some( self.pop_item() )
		}
	}
}

impl<T> ops::Drop for MoveItems<T>
{
	fn drop(&mut self)
	{
		for _ in (self.ofs .. self.count) {
			self.pop_item();
		}
	}
}

// vim: ft=rust
