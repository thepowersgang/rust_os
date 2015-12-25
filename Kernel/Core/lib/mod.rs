// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mod.rs
//!
//! Contains helper types that either clone types in the rust standard library, or provide useful
//! features for operation in kernel-land.

pub use self::queue::Queue;
pub use self::vec_map::VecMap;
//pub use self::btree_map::BTreeMap;
pub use self::vec::Vec;
pub use self::sparse_vec::SparseVec;
pub use self::string::String;
pub use self::lazy_static::LazyStatic;

pub mod thunk;
pub mod borrow;

pub mod ascii;

#[macro_use]
pub mod lazy_static;

pub mod mem;
#[macro_use]
pub mod queue;
#[macro_use]
pub mod vec;
pub mod sparse_vec;

#[macro_use]
pub mod string;
pub mod byte_str;

pub mod vec_map;
//pub mod btree_map;

pub mod ring_buffer;

mod stack_dst_ {
	extern crate stack_dst;
}
pub use self::stack_dst_::stack_dst;

pub mod io;
pub mod byteorder;

mod pod;

pub mod num
{
	//! General numeric helpers
	use core::ops;
	
	pub trait Int
	where
		Self: ops::Add<Output=Self>,
		Self: ops::Sub<Output=Self>,
		Self: ops::Mul<Output=Self>,
		Self: ops::Div<Output=Self>,
		Self: ops::Rem<Output=Self>,
		Self: Sized
	{
		fn one() -> Self;
	}
	impl Int for u64 {
		fn one() -> Self { 1 }
	}
	impl Int for u32 {
		fn one() -> Self { 1 }
	}
	impl Int for usize {
		fn one() -> Self { 1 }
	}
	
	/// Round the passed value up to a multiple of the target value
	pub fn round_up<T: Int+Copy>(val: T, target: T) -> T
	{
		return (val + target - Int::one()) / target * target;
	}
	/// Divide `num` by `den`, rounding up
	pub fn div_up<T: Int+Copy>(num: T, den: T) -> T
	{
		return (num + den - Int::one()) / den;
	}
	/// Divide+Remainder `num` by `den`
	pub fn div_rem<T: Int+Copy>(num: T, den: T) -> (T,T)
	{
		return (num / den, num % den);
	}
}

pub mod collections
{
	//! Collection traits
	
	/// A mutable sequence
	pub trait MutableSeq<T>
	{
		fn push(&mut self, t: T);
		fn pop(&mut self) -> ::core::option::Option<T>;
	}
}

/// Unsafely cast a byte slice into the destination type (performing checks for alignment and size)
///
/// Unsafe because it can't check the validity of the byte representation
pub unsafe fn unsafe_cast_slice<DstType>(src: &[u8]) -> &[DstType]
{
	assert_eq!(src.len() % ::core::mem::size_of::<DstType>(), 0);
	assert_eq!(src.as_ptr() as usize % ::core::mem::align_of::<DstType>(), 0);
	let newlen = src.len() / ::core::mem::size_of::<DstType>();
	::core::slice::from_raw_parts(src.as_ptr() as *const DstType, newlen)
}


/// Unsiged integer bit-level access
pub trait UintBits
{
	/// Returns the value of a single bit
	fn bit(&self, idx: u8) -> Self;
	/// Returns a range of bits (idx .. idx2)
	fn bits(&self, idx: u8, idx2: u8) -> Self;
}

impl UintBits for u16 {
	fn bit(&self, idx: u8) -> u16 {
		(*self >> idx as usize) & 1
	}
	fn bits(&self, idx: u8, idx2: u8) -> u16 {
		(*self >> idx as usize) & ((1 << (idx2 - idx) as usize)-1)
	}
}

/// Printing helper for raw strings
pub struct RawString<'a>(pub &'a [u8]);

impl<'a> ::core::fmt::Debug for RawString<'a>
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		try!(write!(f, "b\""));
		for &b in self.0
		{
			match b
			{
			b'\\' => try!(write!(f, "\\\\")),
			b'\n' => try!(write!(f, "\\n")),
			b'\r' => try!(write!(f, "\\r")),
			b'"' => try!(write!(f, "\\\"")),
			b'\0' => try!(write!(f, "\\0")),
			// ASCII printable characters
			32...127 => try!(write!(f, "{}", b as char)),
			_ => try!(write!(f, "\\x{:02x}", b)),
			}
		}
		try!(write!(f, "\""));
		::core::result::Result::Ok( () )
	}
}


pub struct SlicePtr<'a,T:'a>(pub &'a [T]);
impl<'a,T> ::core::fmt::Pointer for SlicePtr<'a,T> {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		let p = self.0.as_ptr();
		let s = self.0.len();
		write!(f, "({:p}+{})", p, s)
	}
}


pub use self::pod::{POD, as_byte_slice, as_byte_slice_mut};

/// Zip adapter for ExactSizeIterator (easier for the optimiser)
pub struct ExactZip<A,B>(usize,A,B);
impl<A,B> ExactZip<A,B>
where
	A: ExactSizeIterator, B: ExactSizeIterator
{
	pub fn new(a: A, b: B) -> ExactZip<A,B> {
		let size = ::core::cmp::min(a.len(), b.len());
		ExactZip(size, a, b)
	}
}
impl<A,B> Iterator for ExactZip<A,B>
where
	A: ExactSizeIterator, B: ExactSizeIterator
{
	type Item = (A::Item, B::Item);
	fn next(&mut self) -> Option<(A::Item, B::Item)> {
		if self.0 == 0 {
			None
		}
		else {
			self.0 -= 1;
			Some( (self.1.next().unwrap(), self.2.next().unwrap()) )
		}
	}
}

// vim: ft=rust

