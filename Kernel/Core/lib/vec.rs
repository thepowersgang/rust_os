//
//
//
use core::iter::range;
use core::iter::{FromIterator,Iterator};
use core::slice::{SlicePrelude,AsSlice,Items,MutItems};
use core::option::{Option,Some,None};
use core::ptr::RawPtr;
use core::num::Int;
use core::ops::{Drop,Index};
use lib::clone::Clone;
use lib::collections::{MutableSeq};

pub struct Vec<T>
{
	data: *mut T,
	size: uint,
	capacity: uint,
}

pub struct MoveItems<T>
{
	data: *mut T,
	count: uint,
	ofs: uint,
}

impl<T> Vec<T>
{
	pub fn new() -> Vec<T>
	{
		Vec::with_capacity(0)
	}
	pub fn with_capacity(size: uint) -> Vec<T>
	{
		Vec {
			data: unsafe { ::memory::heap::alloc_array::<T>( size ) },
			size: 0,
			capacity: size,
		}
	}
	pub fn from_fn(length: uint, op: |uint| -> T) -> Vec<T>
	{
		let mut ret = Vec::with_capacity(length);
		for i in range(0, length) {
			ret.push( op(i) );
		}
		ret
	}

	pub fn len(&self) -> uint
	{
		self.size
	}
	
	pub fn get_mut<'s>(&'s mut self, index: uint) -> &'s mut T
	{
		if index >= self.size {
			panic!("Index out of range, {} >= {}", index, self.size);
		}
		unsafe { &mut *self.data.offset(index as int) }
	}
	pub fn iter<'s>(&'s self) -> Items<'s,T>
	{
		self.as_slice().iter()
	}
	pub fn iter_mut<'s>(&'s mut self) -> MutItems<'s,T>
	{
		self.slice_mut().iter_mut()
	}
	pub fn into_iter(self) -> MoveItems<T>
	{
		MoveItems {
			data: self.data,
			ofs: 0,
			count: self.size,
		}
	}
	
	fn reserve(&mut self, size: uint)
	{
		let newcap = ::lib::num::round_up(size, 1 << (64-size.leading_zeros()));
		if newcap > self.capacity
		{
			unsafe {
				let newptr = ::memory::heap::alloc_array::<T>( newcap );
				for i in range(0, self.size)
				{
					::core::ptr::write(newptr.offset(i as int), self.move_ent(i as uint));
				}
				if self.capacity > 0 {
					::memory::heap::deallocate( self.data );
				}
				self.data = newptr;
				self.capacity = newcap;
			}
		}
	}
	
	pub fn slice_mut<'a>(&'a mut self) -> &'a mut [T]
	{
		unsafe { ::core::mem::transmute( ::core::raw::Slice { data: self.data as *const T, len: self.size } ) }
	}
	
	/// Move out of a slot in the vector, leaving unitialise memory in its place
	unsafe fn move_ent(&mut self, pos: uint) -> T
	{
		::core::ptr::replace(self.data.offset(pos as int), ::core::mem::uninitialized())
	}
}

impl<T:Clone> Vec<T>
{
	pub fn from_elem(size: uint, elem: T) -> Vec<T>
	{
		Vec::from_fn( size, |_| elem.clone() )
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
		unsafe {
			for i in range(0, self.size) {
				*self.get_mut(i) = ::core::mem::uninitialized();
			}
			::memory::heap::deallocate( self.data );
		}
	}
}

impl<T> Index<uint, T> for Vec<T>
{
	fn index<'a>(&'a self, index: &uint) -> &'a T
	{
		if *index >= self.size {
			panic!("Index out of range, {} >= {}", index, self.size);
		}
		unsafe { &*self.data.offset(*index as int) }
	}
}

impl<T> ::core::slice::AsSlice<T> for Vec<T>
{
	fn as_slice<'a>(&'a self) -> &'a [T]
	{
		let rawslice = ::core::raw::Slice { data: self.data as *const T, len: self.size };
		unsafe { ::core::mem::transmute( rawslice ) }
	}
}

impl<T: ::core::fmt::Show> ::core::fmt::Show for Vec<T>
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::result::Result<(),::core::fmt::Error>
	{
		write!(f, "{}", self.as_slice())
	}
}

impl<T> MutableSeq<T> for Vec<T>
{
	fn push(&mut self, t: T)
	{
		let pos = self.size;
		self.reserve(pos + 1);
		self.size += 1;
		let ptr = self.get_mut(pos);
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
	fn from_iter<IT: Iterator<T>>(mut iterator: IT) -> Vec<T>
	{
		let mut ret = Vec::new();
		for val in iterator
		{
			ret.push(val);
		}
		ret
	}
}

impl<T> MoveItems<T>
{
	fn pop_item(&mut self) -> T
	{
		assert!(self.ofs < self.count);
		let v = unsafe { ::core::ptr::replace(self.data.offset(self.ofs as int), ::core::mem::uninitialized()) };
		self.ofs += 1;
		v
	}
}

impl<T> Iterator<T> for MoveItems<T>
{
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
		for i in range(self.ofs, self.count) {
			self.pop_item();
		}
		unsafe {
			::memory::heap::deallocate( self.data );
		}
	}
}

// vim: ft=rust
