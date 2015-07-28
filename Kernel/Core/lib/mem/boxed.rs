// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mem/mod.rs
//! Owned dynamic allocation (box)
use core::marker::{Sized,Send,Unsize};
use core::ops::{CoerceUnsized};

#[lang = "owned_box"]
pub struct Box<T>(*mut T);

unsafe impl<T: ?Sized+Send> Send for Box<T> { }
impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Box<U>> for Box<T> { }

impl<T> Box<T>
{
	/// Construct a new boxed value (wraps the `box` syntax)
	pub fn new(v: T) -> Box<T> {
		box v
	}
	
	pub unsafe fn into_ptr(self) -> *mut T {
		into_raw(self)
	}
}

pub unsafe fn into_raw<T: ?Sized>(b: Box<T>) -> *mut T {
	::core::mem::transmute(b)
}
pub fn into_inner<T>(b: Box<T>) -> T {
	let box v = b;
	v
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

