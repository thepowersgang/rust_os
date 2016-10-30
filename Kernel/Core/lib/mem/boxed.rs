// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mem/mod.rs
//! Owned dynamic allocation (box)
use core::marker::{Sized,Unsize};
use core::ops::{CoerceUnsized};

#[lang = "owned_box"]
pub struct Box<T: ?Sized>(::core::ptr::Unique<T>);

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Box<U>> for Box<T> { }

impl<T> Box<T>
{
	/// Construct a new boxed value (wraps the `box` syntax)
	pub fn new(v: T) -> Box<T> {
		box v
	}
}
impl<T: ?Sized> Box<T>
{
	pub unsafe fn from_raw(p: *mut T) -> Box<T> {
		//Box(p)
		::core::mem::transmute(p)
	}
	
	pub fn into_ptr(self) -> *mut T {
		self.into_raw()
	}

	pub fn into_raw(self) -> *mut T {
		// SAFE: Leaks 'self', but that's intentional
		unsafe {
			::core::mem::transmute(self)
		}
	}

	pub fn shallow_drop(mut this: Self) {
		// TODO: Is this valid if the inner value has been dropped?
		let size = ::core::mem::size_of_val(&*this);
		let align = ::core::mem::align_of_val(&*this);
		if size != 0 {
			// SAFE: Should be using the correct alignment and size
			unsafe {
				::memory::heap::dealloc_raw(&mut *this as *mut T as *mut (), size, align);
			}
		}
		::core::mem::forget(this);
	}
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

