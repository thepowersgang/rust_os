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
use crate::prelude::*;
use core::sync::atomic::{AtomicUsize,Ordering};
use core::ptr::NonNull;
use core::{ops, fmt};
use core::any::Any;


/// Atomic referencable type. Panics if the type is dropped while any references are active.
/// Internally uses a `Box` to contain an ArefInner
pub struct Aref<T: ?Sized>
{
	__inner: Box<ArefInner<T>>,
}
impl<T: ?Sized + ::core::marker::Unsize<U>, U: ?Sized> ops::CoerceUnsized<Aref<U>> for Aref<T> {}
/// A borrow of an Aref
pub struct ArefBorrow<T: ?Sized>
{
	__ptr: NonNull<ArefInner<T>>,
}
unsafe impl<T: ?Sized + Sync+Send> Send for ArefBorrow<T> {}
unsafe impl<T: ?Sized + Sync+Send> Sync for ArefBorrow<T> {}
impl<T: ?Sized + ::core::marker::Unsize<U>, U: ?Sized> ops::CoerceUnsized<ArefBorrow<U>> for ArefBorrow<T> {}

/// Interior of an Aref. Requires that is is not relocated while any borrows are active
pub struct ArefInner<T: ?Sized>
{
	count: AtomicUsize,
	data: T,
}

impl<T> Aref<T>
{
	/// Construct a new Aref
	pub fn new(val: T) -> Aref<T> {
		Aref {
			// SAFE: Inner is boxed, and cannot be moved out
			__inner: Box::new(unsafe{ ArefInner::new(val) }),
		}
	}
}
	
impl<T: ?Sized> Aref<T>
{
	/// Borrow the `Aref`
	pub fn borrow(&self) -> ArefBorrow<T> {
		self.__inner.borrow()
	}
	/// Obtain a mutable reference to the inner (if unique)
	pub fn get_mut(this: &mut Self) -> Option<&mut T> {
		if this.__inner.count.load(Ordering::SeqCst) == 0 {
			Some(&mut this.__inner.data)
		}
		else {
			None
		}
	}
}
impl<T: ?Sized> ops::Deref for Aref<T>
{
	type Target = T;
	fn deref(&self) -> &T {
		&self.__inner.data
	}
}
impl<T: ?Sized> ops::Drop for Aref<T>
{
	fn drop(&mut self) {
		// SAFE: Constructs a dropped non-Drop value for comparison only
		let cur_count = self.__inner.count.load(Ordering::SeqCst);
		assert!(cur_count == 0, "BUG: Dropping Aref<{}> while {} references are outstanding", type_name!(T), cur_count);
	}
}
impl<T: ?Sized+fmt::Debug> fmt::Debug for Aref<T> {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		fmt::Debug::fmt(&**self, f)
	}
}

impl<T> ArefInner<T>
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
}
impl<T: ?Sized> ArefInner<T>
{
	/// Borrow the inner
	pub fn borrow(&self) -> ArefBorrow<T> {
		self.count.fetch_add(1, Ordering::Relaxed);
		ArefBorrow {
			// SAFE: Pointers are never 0
			__ptr: unsafe { NonNull::new_unchecked(self as *const _ as *mut _) },
			}
	}
}
impl<T: ?Sized> ops::Deref for ArefInner<T>
{
	type Target = T;
	fn deref(&self) -> &T {
		&self.data
	}
}


impl<T: ?Sized> ArefBorrow<T>
{
	pub fn reborrow(&self) -> ArefBorrow<T> {
		self.__inner().borrow()
	}
	fn __inner(&self) -> &ArefInner<T> {
		// SAFE: Nobody gets a &mut to the inner, and pointer should be valid
		unsafe { self.__ptr.as_ref() }
	}
}
impl<T: ?Sized + Any> ArefBorrow<T> {
	pub fn downcast<U: Any>(self) -> Result<ArefBorrow<U>,Self> {
		// SAFE: Transmute validity is checked by checking that the type IDs match
		unsafe { 
			if (*self).type_id() == ::core::any::TypeId::of::<U>() {
				let ptr = self.__ptr.as_ptr() as *const ArefInner<U>;
				::core::mem::forget(self);
				Ok(ArefBorrow { __ptr: NonNull::new_unchecked(ptr as *mut _) })
			}
			else {
				Err(self)
			}
		}
	}
}
impl<T: ?Sized> Clone for ArefBorrow<T>
{
	fn clone(&self) -> Self {
		self.reborrow()
	}
}
impl<T: ?Sized> ops::Deref for ArefBorrow<T>
{
	type Target = T;
	fn deref(&self) -> &T {
		&self.__inner().data
	}
}
impl<T: ?Sized> ops::Drop for ArefBorrow<T>
{
	fn drop(&mut self) {
		// SAFE: Constructs a drop-filled non-Drop type for comparison only
		self.__inner().count.fetch_sub(1, Ordering::Relaxed);
	}
}
