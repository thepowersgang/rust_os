// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mem/rc.rs
//! Reference-counted shared allocation
use core::{ops,fmt};

use super::grc::Grc;

/// Non-atomic reference counted type
pub struct Rc<T: ?Sized> {
	_inner: Grc<::core::cell::Cell<usize>, T>,
}

// Rc is not Send or Sync
impl<T: ?Sized> !Send for Rc<T> {}
impl<T: ?Sized> !Sync for Rc<T> {}
impl<T: ?Sized, U: ?Sized> ops::CoerceUnsized<Rc<U>> for Rc<T> where T: ::core::marker::Unsize<U> {}


impl<T> Rc<T>
{
	/// Create a new Rc
	pub fn new(value: T) -> Rc<T> {
		Rc { _inner: Grc::new(value) }
	}
}
impl<T: ?Sized> Rc<T>
{
	/// Compares this Rc with another, checking if they point to the same object
	pub fn is_same(&self, other: &Rc<T>) -> bool {
		self._inner.is_same( &other._inner )
	}
}

impl<T: ?Sized> Clone for Rc<T> {
	fn clone(&self) -> Rc<T> {
		Rc { _inner: self._inner.clone() }
	}
}

impl<T: ?Sized + fmt::Display> fmt::Display for Rc<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		<T as fmt::Display>::fmt(&**self, f)
	}
}
impl<T: ?Sized + fmt::Debug> fmt::Debug for Rc<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		<T as fmt::Debug>::fmt(&**self, f)
	}
}

impl<T: ?Sized> ops::Deref for Rc<T> {
	type Target = T;
	fn deref(&self) -> &T {
		&*self._inner
	}
}
impl<U> Rc<[U]> {
	/// Construct an Rc'd slice from an iterator
	pub fn from_iter<I>(iterator: I) -> Self
	where
		I: IntoIterator<Item=U>,
		I::IntoIter: ExactSizeIterator,
	{
		Rc { _inner: Grc::from_iter(iterator) }
	}
}
//impl<U> Default for Rc<[U]> {
//	fn default() -> Self {
//		Rc { _inner: Grc::default() }
//	}
//}


// vim: ft=rust

