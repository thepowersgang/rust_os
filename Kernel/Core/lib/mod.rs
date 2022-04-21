// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mod.rs
//!
//! Contains helper types that either clone types in the rust standard library, or provide useful
//! features for operation in kernel-land.

pub use self::queue::Queue;
pub use self::collections::vec_map::VecMap;
//pub use self::btree_map::BTreeMap;
pub use self::vec::Vec;
pub use self::sparse_vec::SparseVec;
pub use self::string::String;
pub use self::lazy_static::LazyStatic;
pub use self::vec_deque::VecDeque;
pub use self::pod::{POD, PodHelpers};

pub use self::pod::{as_byte_slice, as_byte_slice_mut};


pub use self::collections::vec_map;
pub mod collections;

//pub mod thunk;
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
pub mod vec_deque;

#[macro_use]
pub mod string;
pub mod byte_str;

//pub mod btree_map;

pub mod ring_buffer;

pub mod fdt;

pub mod hwreg;

pub extern crate stack_dst;

pub mod io;
pub mod byteorder;

mod pod;

pub mod num;

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
		write!(f, "b\"")?;
		for &b in self.0
		{
			match b
			{
			b'\\' => write!(f, "\\\\")?,
			b'\n' => write!(f, "\\n")?,
			b'\r' => write!(f, "\\r")?,
			b'"'  => write!(f, "\\\"")?,
			b'\0' => write!(f, "\\0")?,
			// ASCII printable characters
			32..=127 => write!(f, "{}", b as char)?,
			_ => write!(f, "\\x{:02x}", b)?,
			}
		}
		write!(f, "\"")?;
		::core::result::Result::Ok( () )
	}
}

pub struct FmtSlice<'a, T: 'a>(pub &'a [T]);
impl<'a, T: 'a + ::core::fmt::LowerHex> ::core::fmt::LowerHex for FmtSlice<'a, T> {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		if self.0.len() == 0 {
			Ok( () )
		}
		else if self.0.len() == 1 {
			self.0[0].fmt(f)
		}
		else {
			self.0[0].fmt(f)?;
			for e in &self.0[1..] {
				f.write_str(",")?;
				e.fmt(f)?;
			}
			Ok( () )
		}
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

