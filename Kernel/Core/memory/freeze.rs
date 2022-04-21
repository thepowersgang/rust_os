// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/freeze.rs
//! Borrow checking/enforcement for the user-kernel boundary
//!
//! Works by marking the referenced pages as being frozen, and either kernel-only (for mut) or read-only (for non-mut)
//! A separate record is stored that contains finer-grained information, handling the case when frozen regions may overlap
//! or at least share a page.
//!
//! The page-fault handler should handle the case of a user PF on a frozen page by sleeping that thread until the page is unfrozen.
#[allow(unused_imports)]
use crate::prelude::*;


#[derive(Debug)]
pub enum FreezeError {
	/// Passed object pointer was into unmapped memory
	Unmapped,
	/// The pased pointer was inaccessible (read-only)
	Inaccessible,
	/// Collides with an existing freeze owned by this thread
	///
	/// TODO: What happens if two threads collide? Should the kernel let them hold each other?
	Locked,
}

/// Type that holds an object in memory, ensuring that it's unmodified and kept valid
pub struct Freeze<T:?Sized>(*const T);

/// Type that holds an object in memory, ensuring that nothing attempts to mutate it
pub struct FreezeMut<T:?Sized>(*mut T);

impl<T: ?Sized> Freeze<T> {
	// UNSAFE: Requires the passed pointer to never alias pointers not protected via the API
	pub unsafe fn new(ptr: *const T) -> Result<Freeze<T>,FreezeError> {
		// TODO: Freeze page as immutable (using a per-process freeze list to handle overlaps)
		Ok( Freeze(ptr) )
	}
}
impl<T: ?Sized> ::core::convert::AsRef<T> for Freeze<T> {
	fn as_ref(&self) -> &T {
		&**self
	}
}
impl<T: ?Sized> ::core::ops::Deref for Freeze<T> {
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: Type ensures that memory is always valid, borrow rules are maintained if construction is valid
		unsafe { &*self.0 }
	}
}

impl<T: ?Sized> FreezeMut<T> {
	// UNSAFE: Requires the passed pointer to never alias pointers not protected via the API
	pub unsafe fn new(ptr: *mut T) -> Result<FreezeMut<T>,FreezeError> {
		// TODO: Freeze page as mutable (using a per-process freeze list to handle overlaps)
		Ok( FreezeMut(ptr) )
	}
}
impl<T: ?Sized> ::core::convert::AsRef<T> for FreezeMut<T> {
	fn as_ref(&self) -> &T {
		&**self
	}
}
impl<T: ?Sized> ::core::ops::Deref for FreezeMut<T> {
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: Type ensures that memory is always valid, borrow rules are maintained if construction is valid
		unsafe { &*self.0 }
	}
}
impl<T: ?Sized> ::core::convert::AsMut<T> for FreezeMut<T> {
	fn as_mut(&mut self) -> &mut T {
		&mut **self
	}
}
impl<T: ?Sized> ::core::ops::DerefMut for FreezeMut<T> {
	fn deref_mut(&mut self) -> &mut T {
		// SAFE: Type ensures that memory is always valid, borrow rules are maintained if construction is valid
		unsafe {&mut *self.0 }
	}
}
