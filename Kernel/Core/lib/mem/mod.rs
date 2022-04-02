// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mem/mod.rs
//! Memory allocation types
pub use self::rc::Rc;
pub use self::arc::Arc;
pub use self::boxed::Box;

mod grc;
pub mod rc;
pub mod arc;

pub mod aref;

pub mod boxed {
	pub use alloc::boxed::Box;
}

#[allow(improper_ctypes)]
extern "C" {
	/// C's `memset` function, VERY UNSAFE
	pub fn memset(dst: *mut u8, val: u8, count: usize);
}


pub struct Unique<T: ?Sized>
{
	p: ::core::ptr::NonNull<T>,
	_pd: ::core::marker::PhantomData<T>,
}
unsafe impl<T: ?Sized + Sync> Sync for Unique<T> {
}
unsafe impl<T: ?Sized + Send> Send for Unique<T> {
}
impl<T: ?Sized> Unique<T>
{
	pub unsafe fn new_unchecked(p: *mut T) -> Unique<T>
	{
		Unique {
			p: ::core::ptr::NonNull::new_unchecked(p),
			_pd: ::core::marker::PhantomData,
			}
	}

	pub fn as_ptr(&self) -> *mut T {
		self.p.as_ptr()
	}
	pub unsafe fn as_ref(&self) -> &T {
		self.p.as_ref()
	}
}

// vim: ft=rust

