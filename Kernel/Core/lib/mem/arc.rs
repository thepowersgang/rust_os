// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mem/arc.rs
//! Atomic reference-counted shared allocation
use _common::*;
use core::nonzero::NonZero;
use core::atomic::{AtomicUsize,Ordering};
use core::{ops,fmt};

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

impl<T> Arc<T>
{
	/// Create a new atomic reference counted object
	pub fn new(value: T) -> Arc<T>
	{
		Arc {
			inner: unsafe { NonZero::new( ArcInner::new_ptr(value) ) },
		}
	}
	
	fn inner(&self) -> &ArcInner<T> {
		unsafe { &**self.inner }
	}
}
impl<T: Clone> Arc<T>
{
	/// Ensure that this instance is the only instance (cloning if needed)
	// &mut self ensures that if the ref count is 1, we can do whatever we want
	pub fn make_unique(&mut self) -> &mut T
	{
		if self.inner().count.load(Ordering::SeqCst) == 1
		{
			// We're the only reference!
		}
		else
		{
			*self = Arc::new( self.inner().val.clone() );
		}
		
		assert!(self.inner().count.load(Ordering::Relaxed) == 1);
		// Obtain a mutable pointer to the interior
		let mut_ptr = *self.inner as *mut ArcInner<T>;
		unsafe { &mut (*mut_ptr).val }
	}
}
impl<T> Clone for Arc<T>
{
	fn clone(&self) -> Arc<T>
	{
		self.inner().count.fetch_add(1, Ordering::Acquire);
		Arc {
			inner: self.inner
		}
	}
}
impl<T: Default> Default for Arc<T> {
	fn default() -> Arc<T> {
		Arc::new( T::default() )
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
		assert!(*self.inner != ::memory::heap::ZERO_ALLOC as *const _);
		unsafe
		{
			let oldcount = (**self.inner).count.fetch_sub(1, Ordering::Release);
			if oldcount == 1
			{
				drop( ::core::ptr::read( &(**self.inner).val ) );
				::memory::heap::dealloc(*self.inner as *mut ArcInner<T>);
			}
			self.inner = NonZero::new( ::memory::heap::ZERO_ALLOC as *const _ );
		}
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

impl<T> ArcInner<T>
{
	unsafe fn new_ptr(value: T) -> *mut ArcInner<T>
	{
		return ::memory::heap::alloc( ArcInner {
			count: AtomicUsize::new(1),
			val: value
			} );
	}
}
