// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mem/rc.rs
//! Reference-counted shared allocations
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

/// Atomic reference-counted type
pub struct Arc<T>
{
	inner: NonZero<*const ArcInner<T>>,
}
// Send if internals are Send
unsafe impl<T: Send> Send for Arc<T> {}
// Sync if internals are Sync
unsafe impl<T: Sync> Sync for Arc<T> {}

struct ArcInner<T>
{
	count: AtomicUsize,
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
		unsafe
		{
			(**self.inner).count -= 1;
			if (**self.inner).count == 0
			{
				drop( ::core::ptr::read( &(**self.inner).val ) );
				::memory::heap::dealloc(*self.inner);
			}
			//self.inner = 0 as *mut _;
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

impl<T> Arc<T>
{
	/// Create a new atomic reference counted object
	pub fn new(value: T) -> Arc<T>
	{
		Arc {
			inner: unsafe { NonZero::new( ::memory::heap::alloc( ArcInner {
				count: AtomicUsize::new(1),
				val: value
				} ) ) },
		}
	}
}
impl<T> Clone for Arc<T>
{
	fn clone(&self) -> Arc<T>
	{
		unsafe {
			(**self.inner).count.fetch_add(1, Ordering::Acquire);
		}
		Arc {
			inner: self.inner
		}
	}
}

impl<T: fmt::Display> fmt::Display for Arc<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		<T as fmt::Display>::fmt(&**self, f)
	}
}
impl<T: fmt::Debug> fmt::Debug for Arc<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		<T as fmt::Debug>::fmt(&**self, f)
	}
}

#[unsafe_destructor]
impl<T> ::core::ops::Drop for Arc<T>
{
	fn drop(&mut self)
	{
		unsafe
		{
			let oldcount = (**self.inner).count.fetch_sub(1, Ordering::Release);
			if oldcount == 1
			{
				drop( ::core::ptr::read( &(**self.inner).val ) );
				::memory::heap::dealloc(*self.inner as *mut ArcInner<T>);
			}
		}
		//self.inner = 0 as *const _;
	}
}

impl<T> ::core::ops::Deref for Arc<T>
{
	type Target = T;
	fn deref<'s>(&'s self) -> &'s T
	{
		unsafe { &(**self.inner).val }
	}
}

// vim: ft=rust

