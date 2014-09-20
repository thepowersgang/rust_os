//
//
//
pub use core::iter::range;
pub use core::option::{Option,Some,None};
pub use core::ptr::RawPtr;
pub use core::num::Int;
pub use core::ops::{Drop,Index};
pub use lib::clone::Clone;

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
		ret.size = length;
		for i in range(0, length) {
			*ret.get_mut(i) = op(i);
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
	
	fn reserve(&mut self, size: uint)
	{
		let newcap = ::lib::num::round_up(size, 1 << (64-size.leading_zeros()));
		if newcap > self.capacity
		{
			unsafe {
				let newptr = ::memory::heap::alloc_array::<T>( newcap );
				for i in range(0, self.size as int)
				{
					*newptr.offset(i) = self.move_ent(i as uint);
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
			unsafe { ::memory::heap::deallocate( self.data ); }
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

impl<T> ::lib::collections::MutableSeq<T> for Vec<T>
{
	fn push(&mut self, t: T)
	{
		let pos = self.size;
		self.size += 1;
		let newsize = self.size;
		self.reserve(newsize);
		*self.get_mut(pos) = t;
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

// vim: ft=rust
