// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mem/rc.rs
//! Reference-counted shared allocation
use core::prelude::*;
use core::nonzero::NonZero;
use core::atomic::{AtomicUsize,Ordering};
use core::{ops,fmt};

/// Non-atomic reference counted type
pub struct Rc<T: ?Sized> {
	_inner: Grc<T, ::core::cell::Cell<usize>>,
}

// Rc is not Send or Sync
impl<T: ?Sized> !Send for Rc<T> {}
impl<T: ?Sized> !Sync for Rc<T> {}

#[doc(hidden)]
pub trait Counter {
	fn zero() -> Self;
	fn one() -> Self;
	fn is_zero(&self) -> bool;
	fn is_one(&self) -> bool;
	fn inc(&self);
	fn dec(&self) -> bool;
}
#[doc(hidden)]
pub struct Grc<T: ?Sized, C: Counter> {
	ptr: NonZero<*mut GrcInner<T,C>>
}
struct GrcInner<T: ?Sized, C: Counter> {
	strong: C,
	//weak: C,
	val: T,
}

impl Counter for ::core::cell::Cell<usize> {
	fn zero() -> Self { ::core::cell::Cell::new(0) }
	fn one() -> Self { ::core::cell::Cell::new(0) }
	fn is_zero(&self) -> bool { self.get() == 0 }
	fn is_one(&self) -> bool { self.get() == 1 }
	fn inc(&self) { self.set( self.get() + 1 ) }
	fn dec(&self) -> bool { self.set( self.get() - 1 ); self.is_zero() }
}
impl Counter for AtomicUsize {
	fn zero() -> Self { AtomicUsize::new(0) }
	fn one() -> Self { AtomicUsize::new(1) }
	fn is_zero(&self) -> bool { self.load(Ordering::SeqCst) == 0 }
	fn is_one(&self) -> bool { self.load(Ordering::SeqCst) == 1 }
	fn inc(&self) { self.fetch_add(1, Ordering::Acquire); }
	fn dec(&self) -> bool { self.fetch_sub(1, Ordering::Acquire) == 1 }
}

impl<T> Rc<T>
{
	/// Create a new Rc
	pub fn new(value: T) -> Rc<T> {
		Rc { _inner: Grc::new(value) }
	}
}
impl<T: ?Sized> Rc<T>
{
	/// Compares this Rc with another, checking if they point to the same object
	pub fn is_same(&self, other: &Rc<T>) -> bool {
		*self._inner.ptr == *other._inner.ptr
	}
}

impl<T: ?Sized> Clone for Rc<T> {
	fn clone(&self) -> Rc<T> {
		Rc { _inner: self._inner.clone() }
	}
}

impl<T: ?Sized> ops::Deref for Rc<T> {
	type Target = T;
	fn deref(&self) -> &T {
		&*self._inner
	}
}
impl<U> Rc<[U]> {
	/// Construct an Rc'd slice from an iterator
	pub fn from_iter<I>(iterator: I) -> Self
	where
		I: IntoIterator<Item=U>,
		I::IntoIter: ExactSizeIterator,
	{
		Rc { _inner: Grc::from_iter(iterator) }
	}
}


impl<T, C:Counter> Grc<T,C>
{
	pub fn new(value: T) -> Grc<T,C> {
		unsafe {
			Grc {
				ptr: NonZero::new( GrcInner::new_ptr(value) )
			}
		}
	}
}
impl<T: ?Sized, C: Counter> Grc<T,C> {
	fn grc_inner(&self) -> &GrcInner<T,C> {
		unsafe { &**self.ptr }
	}
	pub fn get_mut(&mut self) -> Option<&mut T> {
		if self.grc_inner().strong.is_one() {
			Some( unsafe { &mut (*(*self.ptr as *mut GrcInner<T,C>)).val } ) 
		}
		else {
			None
		}
	}
}
impl<T: Default, C: Counter> Default for Grc<T,C> {
	fn default() -> Grc<T,C> {
		Grc::new( T::default() )
	}
}
impl<T: ?Sized, C: Counter> Clone for Grc<T,C>
{
	fn clone(&self) -> Grc<T,C> {
		self.grc_inner().strong.inc();
		Grc { ptr: self.ptr }
	}
}
impl<T: ?Sized + Clone, C: Counter> Grc<T,C>
{
	// &mut self ensures that if the ref count is 1, we can do whatever we want
	pub fn make_unique(&mut self) -> &mut T
	{
		if self.grc_inner().strong.is_one() {
			// We're the only reference!
		}
		else {
			*self = Grc::new( self.grc_inner().val.clone() );
		}
		
		assert!(self.grc_inner().strong.is_one());
		// Obtain a mutable pointer to the interior
		let mut_ptr = *self.ptr as *mut GrcInner<T,C>;
		unsafe { &mut (*mut_ptr).val }
	}
}
impl<T: ?Sized, C: Counter> ops::Deref for Grc<T, C> {
	type Target = T;
	fn deref(&self) -> &T {
		unsafe { &(**self.ptr).val }
	}
}
impl<T: fmt::Display, C: Counter> fmt::Display for Grc<T,C> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		<T as fmt::Display>::fmt(&**self, f)
	}
}
impl<T: fmt::Debug, C: Counter> fmt::Debug for Grc<T,C> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		<T as fmt::Debug>::fmt(&**self, f)
	}
}

impl<T: ?Sized, C: Counter> ops::Drop for Grc<T,C>
{
	fn drop(&mut self)
	{
		//assert!(*self.inner != ::memory::heap::ZERO_ALLOC as *mut _);
		unsafe
		{
			use core::intrinsics::drop_in_place;
			use core::mem::{size_of_val,min_align_of_val};
			let ptr = *self.ptr;
			if (*ptr).strong.dec()
			{
				drop_in_place( &mut (*ptr).val );
				::memory::heap::dealloc_raw(ptr as *mut (), size_of_val(&*ptr), min_align_of_val(&*ptr));
			}
		}
	}
}

impl<T,C: Counter> GrcInner<T,C>
{
	unsafe fn new_ptr(value: T) -> *mut GrcInner<T,C>
	{
		::memory::heap::alloc( GrcInner {
			strong: C::one(),
			//weak: C::zero(),
			val: value,
			} )
	}
}

impl<U, C: Counter> Grc<[U], C>
{
	fn rcinner_align() -> usize {
		::core::cmp::max(::core::mem::align_of::<U>(), ::core::mem::align_of::<usize>())
	}
	unsafe fn rcinner_ptr(count: usize, ptr: *mut ()) -> *mut GrcInner<[U],C> {
		::core::mem::transmute(::core::raw::Slice {
			data: ptr,
			len: count,
			} )
	}
	fn rcinner_size(len: usize) -> usize {
		unsafe {
			let ptr = Self::rcinner_ptr(len, 0 as *mut ());
			::core::mem::size_of_val(&*ptr)
		}
	}
	
	pub fn from_iter<T>(iterator: T) -> Self
	where
		T: IntoIterator<Item=U>,
		T::IntoIter: ExactSizeIterator,
	{
		let it = iterator.into_iter();
		let len = it.len();
		
		let align = Self::rcinner_align();
		let size = Self::rcinner_size(len);
		
		unsafe {
			let ptr = ::memory::heap::alloc_raw(size, align);
			let inner = Self::rcinner_ptr(len, ptr);
			::core::ptr::write( &mut (*inner).strong, C::one() );
			//::core::ptr::write( &mut (*inner).weak, C::zero() );
			for (i,v) in it.enumerate() {
				::core::ptr::write( (*inner).val.as_mut_ptr().offset(i as isize), v );
			}
			
			Grc { ptr: NonZero::new(inner) }
		}
	}
}

// vim: ft=rust

