// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/borrow.rs
//! Borrowed value manipulations (copy of ::std::borrow).
use prelude::*;

/// A copy-on-write value
pub enum Cow<'a, B: ?Sized>
where
	B: 'a,
	B: ToOwned
{
	/// Immutably borrowed value
	Borrowed(&'a B),
	/// Owned value
	Owned(<B as ToOwned>::Owned),
}

pub enum MaybeOwned<'a, B, O>
where
	B: 'a,
	O: Borrow<B>,
{
	Borrowed(&'a B),
	Owned(O),
}

/// Trait for borrowed data that can be cloned into owned data
pub trait ToOwned
where
	<Self as ToOwned>::Owned: Sized,
	<Self as ToOwned>::Owned: Borrow<Self>
{
	/// Owned data type
	type Owned;
	
	/// Create owned data from borrowed data (usually by cloning)
	fn to_owned(&self) -> <Self as ToOwned>::Owned;
}

/// Trait for types that can be borrowed into a different type
pub trait Borrow<B: ?Sized>
{
	fn borrow(&self) -> &B;
}

impl<T: ?Sized> Borrow<T> for T { fn borrow(&self) -> &T { self } }
impl<'a, T: ?Sized> Borrow<T> for &'a T { fn borrow(&self) -> &T { *self } }
impl<'a, T: ?Sized> Borrow<T> for &'a mut T { fn borrow(&self) -> &T { *self } }

//impl<T, U, V> Borrow<T> for V
//where
//	V: Borrow<U>,
//	U: Borrow<T>
//{
//	fn borrow(&self) -> &T {
//		<V as Borrow<U>>::borrow(self).borrow()
//	}
//}

impl<'a, B: 'a + ?Sized + ToOwned> ::core::ops::Deref for Cow<'a, B>
{
	type Target = B;
	fn deref(&self) -> &B
	{
		match self
		{
		&Cow::Borrowed(b) => b,
		&Cow::Owned(ref v) => v.borrow(),
		}
	}
}


// -- Implementations for core types ---
impl<T> Borrow<[T]> for Vec<T>
{
	fn borrow(&self) -> &[T] { &**self }
}
impl<T> ToOwned for [T]
where
	T: Clone
{
	type Owned = Vec<T>;
	fn to_owned(&self) -> Vec<T>
	{
		self.iter().cloned().collect()
	}
}

