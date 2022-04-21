// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/vec.rs
//! Dynamically growable vector type
use crate::prelude::*;
use core::iter::{FromIterator};
use core::{ops,fmt};
use crate::lib::collections::{MutableSeq};
use crate::memory::heap::ArrayAlloc;

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
	/// Create a new vector
	pub const fn new() -> Vec<T>
	{
		Vec {
			data: ArrayAlloc::empty(),
			size: 0,
			}
	}

	/// Create a new vector (constant)
	///
	/// `Vec::new` can't be const because ArrayAlloc::new isn't const
	pub const fn new_const() -> Vec<T> {
		Vec {
			data: ArrayAlloc::empty(),
			size: 0,
			}
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
		for i in 0 .. length {
			ret.push( op(i) );
		}
		ret
	}
	/// 
	pub fn from_boxed_slice(mut s: /*::lib::mem::*/Box<[T]>) -> Vec<T> {
		let ptr = s.as_mut_ptr();
		let len = s.len();
		::core::mem::forget(s);
		Vec {
			// SAFE: All heap pointers are equivalent
			data: unsafe { ArrayAlloc::from_raw(ptr, len) },
			size: len,
		}
	}
	pub fn into_boxed_slice(mut self) -> crate::lib::mem::Box<[T]> {
		let len = self.len();
		self.data.shrink( len );
		// SAFE: (assume) Box<[T]> == *mut [T]
		unsafe {
			let raw: *mut [T] = ::core::ptr::read(&self.data).into_raw();
			::core::mem::forget(self);
			::core::mem::transmute( raw )
		}
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
		::core::mem::forget(self);
		MoveItems {
			data: dataptr,
			ofs: 0,
			count: count,
		}
	}

	/// Clear the vector
	pub fn clear(&mut self)
	{
		self.truncate(0);
	}
	
	fn reserve_cap(&mut self, size: usize)
	{
		let usize_bits: u32 = (::core::mem::size_of::<usize>() * 8) as u32;
		let newcap = crate::lib::num::round_up(size, 1 << (usize_bits - size.leading_zeros()));
		if newcap > self.data.count()
		{
			if self.data.expand(newcap)
			{
				// All good
			}
			else
			{
				let mut newdata = ArrayAlloc::new(newcap);
				// SAFE: Only reads from valid region, writes into invalid data
				unsafe {
					for i in 0 .. self.size {
						let val = self.move_ent(i as usize);
						::core::ptr::write(newdata.get_ptr_mut(i), val);
					}
				}
				//log_debug!("Vec<{}>::reserve_cap({}): newdata={:?}", type_name!(T), size, newdata);
				self.data = newdata;
			}
		}
	}
	
	/// Reserve space in the vector for `extras` new elements
	pub fn reserve(&mut self, extras: usize) {
		let newcap = self.size + extras;
		self.reserve_cap(newcap);
	}
	
	/// Obtain a mutable slice to the content
	pub fn slice_mut<'a>(&'a mut self) -> &'a mut [T]
	{
		// SAFE: self.data is valid up to self.size
		unsafe { ::core::slice::from_raw_parts_mut(self.data.get_base_mut(), self.size) }
	}
	fn as_slice(&self) -> &[T]
	{
		// SAFE: self.data is valid up to self.size
		unsafe { ::core::slice::from_raw_parts( self.data.get_base() as *const T, self.size ) }
	}
	
	/// Move out of a slot in the vector, leaving unitialise memory in its place
	unsafe fn move_ent(&mut self, pos: usize) -> T
	{
		::core::ptr::read(self.data.get_ptr(pos))
	}

	pub fn push(&mut self, value: T)
	{
		let ns = self.size + 1;
		self.reserve_cap(ns);
		// SAFE: Checked accesses
		unsafe
		{
			// Store new element
			::core::ptr::write( self.data.get_ptr_mut(self.size), value );
			self.size += 1;
		}
	}

	pub fn resize_with<F>(&mut self, new_len: usize, mut cb: F)
	where
		F: FnMut() -> T
	{
		if self.len() > new_len {
			self.truncate(new_len);
		}
		else {
			self.reserve_cap(new_len);
			for _ in self.size .. new_len {
				self.push(cb());
			}
		}
	}
	

	/// Insert an item at the specified index (moving subsequent items up)	
	pub fn insert(&mut self, pos: usize, value: T)
	{
		// Expand by one element
		let ns = self.size + 1;
		self.reserve_cap(ns);
		// SAFE: Checked accesses
		unsafe
		{
			// Move elements (pos .. len) to (pos+1 .. len+1) [backwards]
			for i in (pos .. self.size).rev()
			{
				let src = self.data.get_ptr( i );
				let dst = self.data.get_ptr_mut( i+1 );
				::core::ptr::write(dst, ::core::ptr::read(src));
			}
			// Store new element
			::core::ptr::write( self.data.get_ptr_mut(pos), value );
			self.size += 1;
		}
	}
	pub fn remove(&mut self, pos: usize) -> T {
		assert!(pos < self.size);
		// SAFE: Correct pointer accesses
		unsafe
		{
			let rv = ::core::ptr::read( self.data.get_ptr_mut(pos) );
			// Move elements (pos+1 .. len) to (pos .. len-1)
			for i in pos .. self.size-1 {
				let dst = self.data.get_ptr_mut( i );
				let src = self.data.get_ptr( i+1 );
				::core::ptr::write(dst, ::core::ptr::read(src));
			}
			self.size -= 1;
			rv
		}
	}
	
	/// Truncate a vector to the given size
	pub fn truncate(&mut self, newsize: usize)
	{
		if newsize < self.size
		{
			// SAFE: Correct pointer accesses
			unsafe
			{
				for i in newsize .. self.size {
					::core::mem::drop( ::core::ptr::read(self.get_mut_ptr(i) as *const T) );
				}
				self.size = newsize;
			}
		}
	}
}

#[macro_export]
/// Create a new vector in a manner similar array literals
///
/// ```
/// vec![1u32, 2, 3, 4]
/// ```
macro_rules! vec
{
	($( $v:expr ),*) => ({
		$crate::lib::Vec::from_boxed_slice(Box::new([$($v),*]))
		});
	($v:expr; $c:expr) => ({
		$crate::lib::Vec::from_elem($c, $v)
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
	
	pub fn from_elem(size: usize, elem: T) -> Vec<T>
	{
		let mut ret = Vec::with_capacity(size);
		for _ in 0 .. size-1 {
			ret.push(elem.clone());
		}
		ret.push(elem);
		ret
	}
	
	pub fn extend_from_slice(&mut self, other: &[T]) {
		self.reserve(other.len());
		for v in other.iter() {
			self.push(v.clone());
		}
	}
	#[deprecated]
	pub fn push_all(&mut self, other: &[T])
	{
		self.extend_from_slice(other);
	}
}

impl<T> ops::Deref for Vec<T> {
	type Target = [T];
	fn deref(&self) -> &[T] {
		self.as_slice()
	}
}
impl<T> AsRef<[T]> for Vec<T> {
	fn as_ref(&self) -> &[T] {
		self.as_slice()
	}
}
impl<T> ops::DerefMut for Vec<T>
{
	fn deref_mut(&mut self) -> &mut [T] {
		self.slice_mut()
	}
}
impl<T> AsMut<[T]> for Vec<T> {
	fn as_mut(&mut self) -> &mut [T] {
		self.slice_mut()
	}
}
impl<T> ops::Drop for Vec<T>
{
	fn drop(&mut self)
	{
		// SAFE: Drops only values within valid region
		unsafe {
			if ::core::intrinsics::needs_drop::<T>() {
				while self.size > 0 {
					self.size -= 1;
					let idx = self.size;
					let ptr = self.data.get_ptr(idx);
					::core::ptr::drop_in_place( ptr as *mut T );
				}
			}
		}
	}
}

impl<T: Clone> Clone for Vec<T> {
	fn clone(&self) -> Vec<T> {
		self.iter().cloned().collect()
	}
}
impl<'a, T: Clone> From<&'a [T]> for Vec<T> {
	fn from(v: &[T]) -> Vec<T> {
		v.iter().cloned().collect()
	}
}

impl<T> ::core::default::Default for Vec<T>
{
	fn default() -> Vec<T> { Vec::new() }
}

// ---
// Delegating implementations
// ---
impl<T: PartialEq> PartialEq for Vec<T> {
	fn eq(&self, other: &Self) -> bool {
		PartialEq::eq(self.as_slice(), other.as_slice())
	}
}
impl<T: Eq> Eq for Vec<T> {
}
impl<T: PartialOrd> PartialOrd for Vec<T> {
	fn partial_cmp(&self, other: &Self) -> Option<::core::cmp::Ordering> {
		PartialOrd::partial_cmp(self.as_slice(), other.as_slice())
	}
}
impl<T: Ord> Ord for Vec<T> {
	fn cmp(&self, other: &Self) -> ::core::cmp::Ordering {
		Ord::cmp(self.as_slice(), other.as_slice())
	}
}

impl<T: fmt::Debug> fmt::Debug for Vec<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(&**self, f)
	}
}

// ---
// Indexing implementations
// ---
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

impl<T> MutableSeq<T> for Vec<T>
{
	fn push(&mut self, t: T)
	{
		let pos = self.size;
		self.reserve(1);
		self.size += 1;
		let ptr = self.get_mut_ptr(pos);
		// SAFE: Pointer was originally within invalid region
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
			// SAFE: Pointer within old valid region
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

impl<T> ::core::iter::Extend<T> for Vec<T>
{
	fn extend<I: IntoIterator<Item=T>>(&mut self, src: I)
	{
		let iter = src.into_iter();
		
		if let (_, Some(size)) = iter.size_hint()
		{
			self.reserve(size);
		}
		for val in iter {
			self.push(val);
		}
	}
}

impl<T> IntoIterator for Vec<T>
{
	type IntoIter = MoveItems<T>;
	type Item = T;
	fn into_iter(self) -> MoveItems<T> {
		self.into_iter()
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
		// SAFE: Pointer asserted to be within valid range
		let v: T = unsafe {
			assert!(self.ofs < self.count);
			let ptr = self.data.get_ptr(self.ofs);
			::core::ptr::read(ptr as *const _)
			};
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
		for _ in self.ofs .. self.count {
			self.pop_item();
		}
	}
}

// vim: ft=rust
