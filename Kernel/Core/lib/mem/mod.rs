// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mem/mod.rs
//! Memory allocation types
use core::marker::{Sized,Send};

pub use self::rc::Rc;
pub use self::arc::Arc;

mod rc;
mod arc;

/// Owned dynamic allocation (box)
#[lang = "owned_box"]
pub struct Box<T>(*mut T);

unsafe impl<T: ?Sized+Send> Send for Box<T> { }

impl<T> Box<T>
{
	/// Construct a new boxed value (wraps the `box` syntax)
	pub fn new(v: T) -> Box<T> {
		box v
	}
	
	pub unsafe fn into_ptr(self) -> *mut T {
		::core::mem::transmute(self)
	}
}

impl<T: ?Sized> ::core::fmt::Debug for Box<T>
where
	T: ::core::fmt::Debug
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::result::Result<(),::core::fmt::Error>
	{
		(**self).fmt(f)
	}
}

#[allow(improper_ctypes)]
extern "C" {
	#[no_mangle]
	/// C's `memset` function, VERY UNSAFE
	pub fn memset(dst: *mut u8, val: u8, count: usize);
}

// vim: ft=rust

