//
//
//
use core::iter::range;
use core::iter::{FromIterator,Iterator};
use core::slice::{Slice,ImmutableSlice,Items};
use core::option::{Option,Some,None};
use core::ptr::RawPtr;
use core::num::Int;
use core::ops::{Drop,Index};
use lib::clone::Clone;
use core::collections::{Collection};
use lib::collections::{MutableSeq};

pub struct Vec<T>
{
	data: *mut T,
	size: uint,
	capacity: uint,
}

impl<T> Vec<T>
{
	pub fn new() -> Vec<T>
	{
		Vec {
			data: RawPtr::null(),
			size: 0,
			capacity: 0,
		}
	}
	pub fn with_capacity(size: uint) -> Vec<T>
	{
		let mut ret = Vec::new();
		ret.reserve(size);
		ret
	}
	pub fn from_fn(length: uint, op: |uint| -> T) -> Vec<T>
	{
		let mut ret = Vec::with_capacity(length);
		for i in range(0, length) {
			ret.push( op(i) );
		}
		ret
	}

	
	pub fn get_mut<'s>(&'s mut self, index: uint) -> &'s mut T
	{
		if index >= self.size {
			fail!("Index out of range, {} >= {}", index, self.size);
		}
		unsafe { &mut *self.data.offset(index as int) }
	}
	pub fn iter<'s>(&'s self) -> Items<'s,T>
	{
		self.as_slice().iter()
	}
	
	fn reserve(&mut self, size: uint)
	{
		let newcap = ::lib::num::round_up(size, 1 << (64-size.leading_zeros()));
		if newcap > self.capacity
		{
			unsafe {
				let newptr = ::memory::heap::alloc_array::<T>( newcap );
				for i in range(0, self.size as int)
				{
					::core::ptr::write(newptr.offset(i), self.move_ent(i as uint));
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
}

#[unsafe_destructor]
impl<T> Drop for Vec<T>
{
	fn drop(&mut self)
	{
		if self.capacity > 0
		{
			unsafe {
				for i in range(0, self.size) {
					*self.get_mut(i) = ::core::mem::uninitialized();
				}
				::memory::heap::deallocate( self.data );
			}
		}
	}
}

impl<T> Index<uint, T> for Vec<T>
{
	fn index<'a>(&'a self, index: &uint) -> &'a T
	{
		if *index >= self.size {
			fail!("Index out of range, {} >= {}", index, self.size);
		}
		unsafe { &*self.data.offset(*index as int) }
	}
}

impl<T> ::core::slice::Slice<T> for Vec<T>
{
	fn as_slice<'a>(&'a self) -> &'a [T]
	{
		unsafe { ::core::mem::transmute( ::core::raw::Slice { data: self.data as *const T, len: self.size } ) }
	}
}

impl<T> Collection for Vec<T>
{
	fn len(&self) -> uint { self.size }
}

impl<T> MutableSeq<T> for Vec<T>
{
	fn push(&mut self, t: T)
	{
		let pos = self.size;
		self.size += 1;
		let newsize = self.size;
		self.reserve(newsize);
		unsafe { ::core::ptr::write(self.get_mut(pos), t); }
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
	fn from_iter<IT: Iterator<T>>(iterator: IT) -> Vec<T>
	{
		let mut it = iterator;
		let mut ret = Vec::new();
		loop
		{
			match it.next() {
			Some(x) => ret.push(x),
			None => break,
			}
		}
		ret
	}
}

// vim: ft=rust
