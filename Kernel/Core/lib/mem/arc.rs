// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mem/arc.rs
//! Atomic reference-counted shared allocation
use core::sync::atomic::AtomicUsize;
use core::{ops,fmt};

use super::grc::Grc;

/// Atomic reference-counted type
pub struct Arc<T: ?Sized>
{
	_inner: Grc<AtomicUsize, T>,
}
// Send if internals are Send+Sync
unsafe impl<T: ?Sized + Send+Sync> Send for Arc<T> {}
// Sync if internals are Send+Sync (Sync allows &Arc which allows cloning)
unsafe impl<T: ?Sized + Send+Sync> Sync for Arc<T> {}

impl<T> Arc<T>
{
	/// Create a new atomic reference counted object
	pub fn new(value: T) -> Arc<T>
	{
		Arc { _inner: Grc::new(value) }
	}
}


impl<T: ?Sized> Arc<T>
{
	pub fn strong_count(this: &Arc<T>) -> usize {
		this._inner.strong_count()
	}

	/// Returns Some(mut_ref) when this Arc only has one reference
	pub fn get_mut(this: &mut Arc<T>) -> Option<&mut T> {
		this._inner.get_mut()
	}
}
impl<T: Default> Default for Arc<T> {
	fn default() -> Arc<T> {
		Arc::new( T::default() )
	}
}
impl<T: Clone> Arc<T>
{
	/// Ensure that this instance is the only instance (cloning if needed)
	pub fn make_mut(this: &mut Arc<T>) -> &mut T {
		this._inner.make_unique()
	}
}
impl<U> Arc<[U]> {
	pub fn from_fn<F>(count: usize, fcn: F) -> Arc<[U]>//Self
	where
		F: FnMut(usize)->U
	{
		Arc { _inner: Grc::from_fn(count, fcn) }
	}
	/// Construct an Rc'd slice from an iterator
	pub fn from_iter<I>(iterator: I) -> Self
	where
		I: IntoIterator<Item=U>,
		I::IntoIter: ExactSizeIterator,
	{
		Arc { _inner: Grc::from_iter(iterator) }
	}
}
//impl<U> Default for Arc<[U]> {
//	fn default() -> Self {
//		Arc { _inner: Grc::default() }
//	}
//}

impl<T: ?Sized> Clone for Arc<T>
{
	fn clone(&self) -> Arc<T> {
		Arc { _inner: self._inner.clone() }
	}
}

impl<T: ?Sized + fmt::Display> fmt::Display for Arc<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		<T as fmt::Display>::fmt(&**self, f)
	}
}
impl<T: ?Sized + fmt::Debug> fmt::Debug for Arc<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		<T as fmt::Debug>::fmt(&**self, f)
	}
}
impl<T> fmt::Pointer for Arc<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		<_ as fmt::Pointer>::fmt(&self._inner, f)
	}
}

impl<T: ?Sized> ops::Deref for Arc<T>
{
	type Target = T;
	fn deref(&self) -> &T {
		&*self._inner
	}
}

