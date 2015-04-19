// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mem/rc.rs
//! Reference-counted shared allocation
use _common::*;
use core::nonzero::NonZero;
use core::atomic::{AtomicUsize,Ordering};
use core::{ops,fmt};

/// Non-atomic reference counted type
pub struct Rc<T>
{
	inner: NonZero<*mut RcInner<T>>,
}

// Rc is not Send
impl<T> !Send for Rc<T> {}
// Rc is Sync (if the internals are Sync)
unsafe impl<T: Sync> Sync for Rc<T> {}

struct RcInner<T>
{
	count: usize,
	val: T,
}

impl<T> Rc<T>
{
	/// Create a new Rc
	pub fn new(value: T) -> Rc<T>
	{
		unsafe {
			Rc {
				inner: NonZero::new( RcInner::new_ptr(value) )
			}
		}
	}
	/// Compares this Rc with another, checking if they point to the same object
	pub fn is_same(&self, other: &Rc<T>) -> bool {
		*self.inner == *other.inner
	}
}

impl<T> Clone for Rc<T>
{
	fn clone(&self) -> Rc<T>
	{
		unsafe { (**self.inner).count += 1; }
		Rc {
			inner: self.inner
		}
	}
}

impl<T> ::core::ops::Deref for Rc<T>
{
	type Target = T;
	fn deref<'s>(&'s self) -> &'s T
	{
		unsafe { &(**self.inner).val }
	}
}

#[unsafe_destructor]
impl<T> ::core::ops::Drop for Rc<T>
{
	fn drop(&mut self)
	{
		assert!(*self.inner != ::memory::heap::ZERO_ALLOC as *mut _);
		unsafe
		{
			(**self.inner).count -= 1;
			if (**self.inner).count == 0
			{
				drop( ::core::ptr::read( &(**self.inner).val ) );
				::memory::heap::dealloc(*self.inner);
			}
			self.inner = NonZero::new( ::memory::heap::ZERO_ALLOC as *mut _ );
		}
	}
}

impl<T> RcInner<T>
{
	unsafe fn new_ptr(value: T) -> *mut RcInner<T>
	{
		return ::memory::heap::alloc( RcInner {
			count: 1,
			val: value,
			} );
	}
}


// vim: ft=rust

