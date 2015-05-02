// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mod.rs
//!
//! Contains helper types that either clone types in the rust standard library, or provide useful
//! features for operation in kernel-land.
use core::prelude::*;

pub use self::queue::Queue;
pub use self::vec_map::VecMap;
pub use self::btree_map::BTreeMap;
pub use self::vec::Vec;
pub use self::string::String;

pub mod thunk;

pub mod borrow;

pub mod mem;
#[macro_use]
pub mod queue;
#[macro_use]
pub mod vec;
pub mod sparse_vec;

#[macro_use]
pub mod string;

pub mod vec_map;
pub mod btree_map;

pub mod ring_buffer;

//pub mod stack_dsts;

pub mod io;
pub mod byteorder;


pub mod num
{
	//! General numeric helpers
	
	use _common::*;
	use core::ops;
	
	pub trait Int
	where
		Self: ops::Add<Output=Self>,
		Self: ops::Sub<Output=Self>,
		Self: ops::Mul<Output=Self>,
		Self: ops::Div<Output=Self>
	{
		fn one() -> Self;
	}
	impl Int for u64 {
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
	::core::mem::transmute(::core::raw::Slice {
		data: src.as_ptr() as *const DstType,
		len: newlen,
		})
}

/// A lazily initialised value (for `static`s)
pub struct LazyStatic<T: Send+Sync>(pub ::core::cell::UnsafeCell<Option<T>>);
unsafe impl<T: Send+Sync> Sync for LazyStatic<T> {}	// Barring the unsafe "prep" call, is Sync
unsafe impl<T: Send+Sync> Send for LazyStatic<T> {}	// Sendable because inner is sendable

macro_rules! lazystatic_init {
	() => ($crate::lib::LazyStatic($crate::core::cell::UnsafeCell { value: $crate::core::option::Option::None }));
}

impl<T: Send+Sync> LazyStatic<T>
{
	/// (unsafe) Prepare the value using the passed function
	///
	/// Unsafe because it must NOT be called where a race is possible
	pub unsafe fn prep<Fcn: FnOnce()->T>(&self, fcn: Fcn) {
		let r = &mut *self.0.get();
		assert!(r.is_none(), "LazyStatic<{}> initialised multiple times", type_name!(T));
		if r.is_none() {
			*r = Some(fcn());
		}
	}
	pub unsafe fn ls_unsafe_mut(&self) -> &mut T {
		match *self.0.get()
		{
		Some(ref mut v) => v,
		None => panic!("Dereferencing LazyStatic<{}> without initialising", type_name!(T))
		}
	}
}
impl<T: Send+Sync> ::core::ops::Deref for LazyStatic<T>
{
	type Target = T;
	fn deref(&self) -> &T {
		match unsafe { (&*self.0.get()).as_ref() } {
		Some(v) => v,
		None => panic!("Dereferencing LazyStatic<{}> without initialising", type_name!(T))
		}
	}
}

/// An equivalemnt of Option<*const T> which cannot be NULL
pub struct OptPtr<T>(pub *const T);
unsafe impl<T: Send> Send for OptPtr<T> {}
/// An equivalemnt of Option<*mut T> which cannot be NULL
pub struct OptMutPtr<T>(pub *mut T);
unsafe impl<T: Send> Send for OptMutPtr<T> {}

impl<T> OptPtr<T>
{
	pub fn is_none(&self) -> bool {
		self.0.is_null()
	}
	pub fn is_some(&self) -> bool {
		!self.0.is_null()
	}
	pub fn unwrap(&self) -> *const T {
		assert!( !self.0.is_null() );
		self.0
	}
	pub unsafe fn as_ref(&self) -> Option<&T> {
		if (self.0).is_null() {
			None
		}
		else {
			Some(&*self.0)
		}
	}
	pub unsafe fn as_mut(&self) -> OptMutPtr<T> {
		::core::mem::transmute(self)
	}
	/// This is HIGHLY unsafe
	pub unsafe fn as_mut_ref(&self) -> Option<&mut T> {
		::core::mem::transmute(self.as_ref())
	}
}

impl<T> OptMutPtr<T>
{
	pub fn is_none(&self) -> bool {
		self.0.is_null()
	}
	pub fn is_some(&self) -> bool {
		!self.0.is_null()
	}
	pub fn unwrap(&self) -> *mut T {
		assert!( !self.0.is_null() );
		self.0
	}
	pub unsafe fn as_ref(&self) -> Option<&mut T> {
		if (self.0).is_null() {
			None
		}
		else {
			Some(&mut *self.0)
		}
	}
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

// vim: ft=rust

