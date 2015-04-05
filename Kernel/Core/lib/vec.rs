// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/vec.rs
//! Dynamically growable vector type
use core::prelude::*;
use core::num::Int;	// for leading_zeros()
use core::iter::{FromIterator,IntoIterator};
use core::ops::{Index,IndexMut,Deref,DerefMut};
use lib::collections::{MutableSeq};
use core::ptr::Unique;

/// Growable array of items
pub struct Vec<T>
{
	data: Unique<T>,
	size: usize,
	capacity: usize,
}

/// Owning iterator
pub struct MoveItems<T>
{
	data: Unique<T>,
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
			data: unsafe { Unique::new( ::memory::heap::alloc_array::<T>( size ) ) },
			size: 0,
			capacity: size,
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
		if index >= self.size {
			panic!("Index out of range, {} >= {}", index, self.size);
		}
		unsafe { self.data.offset(index as isize) }
	}
	
	/// Move contents into an iterator (consuming self)
	pub fn into_iter(mut self) -> MoveItems<T>
	{
		let dataptr = ::core::mem::replace(&mut self.data, unsafe { Unique::new(0 as *mut _) } );
		let count = self.size;
		unsafe { ::core::mem::forget(self) };
		MoveItems {
			data: dataptr,
			ofs: 0,
			count: count,
		}
	}
	
	fn reserve(&mut self, size: usize)
	{
		let newcap = ::lib::num::round_up(size, 1 << (64-size.leading_zeros()));
		if newcap > self.capacity
		{
			unsafe {
				let newptr = ::memory::heap::alloc_array::<T>( newcap );
				for i in (0 .. self.size) {
					let val = self.move_ent(i as usize);
					::core::ptr::write(newptr.offset(i as isize), val);
				}
				if self.capacity > 0 {
					::memory::heap::dealloc_array( *self.data, self.capacity );
				}
				self.data = Unique::new(newptr);
				self.capacity = newcap;
			}
		}
	}
	
	pub fn slice_mut<'a>(&'a mut self) -> &'a mut [T]
	{
		unsafe { ::core::mem::transmute( ::core::raw::Slice { data: *self.data as *const T, len: self.size } ) }
	}
	
	/// Move out of a slot in the vector, leaving unitialise memory in its place
	unsafe fn move_ent(&mut self, pos: usize) -> T
	{
		::core::ptr::read(self.data.offset(pos as isize) as *const _)
	}

	/// Insert an item at the specified index (moving subsequent items up)	
	pub fn insert(&mut self, pos: usize, value: T)
	{
		// Expand by one element
		let ns = self.size + 1;
		self.reserve(ns);
		self.size = ns;
		unsafe
		{
			// Move elements (pos .. len) to (pos+1 .. len+1)
			for i in (pos .. self.size).rev()
			{
				let src = self.data.offset( (i) as isize);
				let dst = self.data.offset( (i+1) as isize);
				::core::ptr::write(dst, ::core::ptr::read(src));
			}
			// Store new element
			::core::ptr::write( self.data.offset(pos as isize), value );
		}
	}
}

impl<T> Deref for Vec<T>
{
	type Target = [T];
	fn deref(&self) -> &[T] {
		self.as_slice()
	}
}
impl<T> DerefMut for Vec<T>
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
		let newlen = self.size + other.len();
		self.reserve(newlen);
		for v in other.iter() {
			self.push(v.clone());
		}
	}
}

#[unsafe_destructor]
impl<T> Drop for Vec<T>
{
	fn drop(&mut self)
	{
		log_debug!("Vec::drop() - Dropping vector at {:p} w/ {} ents", *self.data, self.size);
		unsafe {
			for i in (0 .. self.size) {
				::core::mem::drop( ::core::ptr::read(self.get_mut_ptr(i) as *const T) );
			}
			::memory::heap::dealloc_array( *self.data, self.capacity );
		}
	}
}

impl<T> Index<usize> for Vec<T>
{
	type Output = T;
	fn index<'a>(&'a self, index: usize) -> &'a T
	{
		&self.as_slice()[index]
	}
}
impl<T> IndexMut<usize> for Vec<T>
{
	fn index_mut<'a>(&'a mut self, index: usize) -> &'a mut T
	{
		&mut self.slice_mut()[index]
	}
}

impl<T> ::core::slice::AsSlice<T> for Vec<T>
{
	fn as_slice<'a>(&'a self) -> &'a [T]
	{
		let rawslice = ::core::raw::Slice { data: *self.data as *const T, len: self.size };
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
		self.reserve(pos + 1);
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
			ret.reserve(size);
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
			let ptr = self.data.offset(self.ofs as isize);
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

#[unsafe_destructor]
impl<T> Drop for MoveItems<T>
{
	fn drop(&mut self)
	{
		for _ in (self.ofs .. self.count) {
			self.pop_item();
		}
		unsafe {
			::memory::heap::dealloc_array( *self.data, self.count );
		}
	}
}

// vim: ft=rust
