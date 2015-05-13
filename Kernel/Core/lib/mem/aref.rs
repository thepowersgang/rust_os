// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mem/aref.rs
//! Atomic reference type (Arc but with weak pointers only)
//!
//! Provides runtime lifetime checking (similar to how RefCell provides runtime borrow checking)
//!
//! This type is designed to be used where there is a definitive owner of a peice of memory (e.g. a box)
//! but you also want to lend pointers to that memory out (where the pointers should never outlive the 
//! original memory).
use prelude::*;
use core::atomic::{AtomicUsize,Ordering};
use core::nonzero::NonZero;
use core::ops;


/// Atomic referencable type. Panics if the type is dropped while any references are active.
/// Internally uses a `Box` to contain an ArefInner
pub struct Aref<T: Sync>
{
	__inner: Box<ArefInner<T>>,
}
/// A borrow of an Aref
pub struct ArefBorrow<T: Sync>
{
	__ptr: NonZero<*const ArefInner<T>>,
}
unsafe impl<T: Sync+Send> Send for ArefBorrow<T> {}

/// Interior of an Aref. Requires that is is not relocated while any borrows are active
pub struct ArefInner<T: Sync>
{
	count: AtomicUsize,
	data: T,
}

impl<T: Sync> Aref<T>
{
	/// Construct a new Aref
	pub fn new(val: T) -> Aref<T> {
		Aref {
			__inner: Box::new(unsafe{ ArefInner::new(val) }),
		}
	}
	
	/// Borrow the `Aref`
	pub fn borrow(&self) -> ArefBorrow<T> {
		self.__inner.borrow()
	}
}
impl<T: Sync> ops::Deref for Aref<T>
{
	type Target = T;
	fn deref(&self) -> &T {
		&self.__inner.data
	}
}

impl<T: Sync> ArefInner<T>
{
	/// Unsafely create a new interior
	///
	/// You MUST ensure that the inner is not moved out of its memory location while any borrows are active
	pub unsafe fn new(val: T) -> ArefInner<T> {
		ArefInner {
			count: AtomicUsize::new(0),
			data: val,
		}
	}
	/// Borrow the inner
	pub fn borrow(&self) -> ArefBorrow<T> {
		self.count.fetch_add(1, Ordering::Relaxed);
		ArefBorrow {
			// SAFE: Pointers are never 0
			__ptr: unsafe { NonZero::new(self) },
			}
	}
}
impl<T: Sync> ops::Deref for ArefInner<T>
{
	type Target = T;
	fn deref(&self) -> &T {
		&self.data
	}
}
impl<T: Sync> ops::Drop for ArefInner<T>
{
	fn drop(&mut self) {
		assert_eq!(self.count.load(Ordering::Relaxed), 0);
	}
}


impl<T: Sync> ArefBorrow<T>
{
	fn __inner(&self) -> &ArefInner<T> {
		unsafe { &**self.__ptr }
	}
}
impl<T: Sync> ops::Deref for ArefBorrow<T>
{
	type Target = T;
	fn deref(&self) -> &T {
		&self.__inner().data
	}
}
impl<T: Sync> ops::Drop for ArefBorrow<T>
{
	fn drop(&mut self) {
		self.__inner().count.fetch_sub(1, Ordering::Relaxed);
	}
}
