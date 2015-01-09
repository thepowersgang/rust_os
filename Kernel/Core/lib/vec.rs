//
//
//
use core::iter::range;
use core::iter::{FromIterator,Iterator};
use core::slice::{SliceExt,AsSlice};
use core::option::Option::{self,Some,None};
use core::ptr::PtrExt;
use core::num::Int;
use core::ops::{Drop,Index,IndexMut,Deref,DerefMut,Fn};
use core::kinds::Send;
use lib::clone::Clone;
use lib::collections::{MutableSeq};

pub struct Vec<T>
{
	data: *mut T,
	size: uint,
	capacity: uint,
}
unsafe impl<T: Send> Send for Vec<T> {}

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
	pub fn from_fn<Fcn>(length: uint, op: Fcn) -> Vec<T>
	where
		Fcn: Fn(uint) -> T
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
	
	pub fn get_mut_ptr(&mut self, index: uint) -> *mut T
	{
		if index >= self.size {
			panic!("Index out of range, {} >= {}", index, self.size);
		}
		unsafe { self.data.offset(index as int) }
	}
	pub fn into_iter(self) -> MoveItems<T>
	{
		let dataptr = self.data;
		let count = self.size;
		unsafe { ::core::mem::forget(self) };
		MoveItems {
			data: dataptr,
			ofs: 0,
			count: count,
		}
	}
	
	fn reserve(&mut self, size: uint)
	{
		let newcap = ::lib::num::round_up(size, 1 << (64-size.leading_zeros()));
		if newcap > self.capacity
		{
			unsafe {
				let newptr = ::memory::heap::alloc_array::<T>( newcap );
				for i in range(0, self.size) {
					let val = self.move_ent(i as uint);
					::core::ptr::write(newptr.offset(i as int), val);
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
		::core::ptr::read(self.data.offset(pos as int) as *const _)
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
		self.as_mut_slice()
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
		log_debug!("Vec::drop() - Dropping vector at {} w/ {} ents", self.data, self.size);
		unsafe {
			for i in range(0, self.size) {
				::core::mem::drop( ::core::ptr::read(self.get_mut_ptr(i) as *const T) );
			}
			::memory::heap::deallocate( self.data );
		}
	}
}

impl<T> Index<uint> for Vec<T>
{
	type Output = T;
	fn index<'a>(&'a self, index: &uint) -> &'a T
	{
		if *index >= self.size {
			panic!("Index out of range, {} >= {}", index, self.size);
		}
		unsafe { &*self.data.offset(*index as int) }
	}
}
impl<T> IndexMut<uint> for Vec<T>
{
	type Output = T;	//< Shouldn't be needed, but ICEs without it
	fn index_mut<'a>(&'a mut self, index: &uint) -> &'a mut T
	{
		if *index >= self.size {
			panic!("Index out of range, {} >= {}", index, self.size);
		}
		unsafe { &mut *self.data.offset(*index as int) }
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
	fn from_iter<IT: Iterator<Item=T>>(mut iterator: IT) -> Vec<T>
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
		//log_debug!("MoveItems.pop_item() ofs={}, count={}, data={}", self.ofs, self.count, self.data);
		assert!(self.ofs < self.count);
		let v: T = unsafe {
			let ptr = self.data.offset(self.ofs as int);
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
		for _ in range(self.ofs, self.count) {
			self.pop_item();
		}
		unsafe {
			::memory::heap::deallocate( self.data );
		}
	}
}

// vim: ft=rust
