// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mem/grc.rs
//! Generic reference-counted shared allocation
//!
//! Provides common functionality between Rc and Arc
use core::ptr::NonNull;
use core::sync::atomic::{AtomicUsize,Ordering};
use core::{ops,fmt};

/// Abstraction crate for the reference counting
pub trait Counter {
	fn zero() -> Self;
	fn one() -> Self;
	fn get(&self) -> usize;
	fn is_zero(&self) -> bool;
	fn is_one(&self) -> bool;
	fn inc(&self);
	fn dec(&self) -> bool;
}

/// Generic reference counted allocation
pub struct Grc<C: Counter, T: ?Sized> {
	ptr: NonNull<GrcInner<C, T>>
}

/// Not Send (Arc overrides this)
impl<C: Counter, T: ?Sized> !Send for Grc<C, T> {}
/// Nor Sync
impl<C: Counter, T: ?Sized> !Sync for Grc<C, T> {}
/// Can be unsized coerced
impl<C: Counter, T: ?Sized, U: ?Sized> ops::CoerceUnsized<Grc<C, U>> for Grc<C, T> where T: ::core::marker::Unsize<U> {}

/// Internals (i.e. the contents of the allocation)
struct GrcInner<C: Counter, T: ?Sized> {
	strong: C,
	//weak: C,
	val: T,
}

/// Interior-mutable unsigned integer (non-atomic)
impl Counter for ::core::cell::Cell<usize> {
	fn zero() -> Self { ::core::cell::Cell::new(0) }
	fn one() -> Self { ::core::cell::Cell::new(0) }
	fn get(&self) -> usize { self.get() }
	fn is_zero(&self) -> bool { self.get() == 0 }
	fn is_one(&self) -> bool { self.get() == 1 }
	fn inc(&self) { self.set( self.get() + 1 ) }
	fn dec(&self) -> bool { self.set( self.get() - 1 ); self.is_zero() }
}
/// Atomic unsigned integer
impl Counter for AtomicUsize {
	fn zero() -> Self { AtomicUsize::new(0) }
	fn one() -> Self { AtomicUsize::new(1) }
	fn get(&self) -> usize { self.load(Ordering::Relaxed) }
	fn is_zero(&self) -> bool { self.load(Ordering::SeqCst) == 0 }
	fn is_one(&self) -> bool { self.load(Ordering::SeqCst) == 1 }
	fn inc(&self) { self.fetch_add(1, Ordering::Acquire); }
	fn dec(&self) -> bool { self.fetch_sub(1, Ordering::Acquire) == 1 }
}

impl<C: Counter, T> Grc<C, T>
{
	/// Sized constructor
	pub fn new(value: T) -> Grc<C, T> {
		// SAFE: Pointer won't be NULL
		unsafe {
			Grc {
				ptr: NonNull::new_unchecked( GrcInner::new_ptr(value) )
			}
		}
	}
}
impl<C: Counter, T: ?Sized> Grc<C, T>
{
	fn grc_inner(&self) -> &GrcInner<C, T> {
		// SAFE: Immutable alias valid since self: &Self
		unsafe { self.ptr.as_ref() }
	}
	pub fn is_same(&self, other: &Self) -> bool {
		self.ptr.as_ptr() == other.ptr.as_ptr()
	}
	pub fn strong_count(&self) -> usize {
		self.grc_inner().strong.get()
	}
	pub fn get_mut(&mut self) -> Option<&mut T> {
		if self.grc_inner().strong.is_one() {
			// SAFE: This instance is the only reference, and we have &mut, hence safe to get &mut to inner
			Some( unsafe { &mut (*(self.ptr.as_ptr() as *mut GrcInner<C,T>)).val } ) 
		}
		else {
			None
		}
	}
}
/// Create an allocation using the interior's default
impl<C: Counter, T: Default> Default for Grc<C, T> {
	fn default() -> Grc<C, T> {
		Grc::new( T::default() )
	}
}
//impl<U, C: Counter> Default for Grc<[U],C> {
//	fn default() -> Self {
//		//Grc { ptr: 
//		Grc::new([])
//	}
//}

/// Create a new shared reference
impl<C: Counter, T: ?Sized> Clone for Grc<C,T>
{
	fn clone(&self) -> Grc<C,T> {
		self.grc_inner().strong.inc();
		Grc { ptr: self.ptr }
	}
}
impl<C: Counter, T: Clone> Grc<C,T>
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
		
		// Obtain a mutable pointer to the interior
		let mut_ptr = self.ptr.as_ptr() as *mut GrcInner<C,T>;
		// SAFE: Can only get &mut if this instance is the only handle
		unsafe {
			assert!(self.grc_inner().strong.is_one());
			&mut (*mut_ptr).val
			}
	}
}
/// Dereferences to interior
impl<C: Counter, T: ?Sized> ops::Deref for Grc<C, T> {
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: Pointer is valid, can't get &mut so no aliasing issues
		unsafe { &self.ptr.as_ref().val }
	}
}
impl<C: Counter, T: ?Sized + fmt::Display> fmt::Display for Grc<C, T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		<T as fmt::Display>::fmt(&**self, f)
	}
}
impl<C: Counter, T: ?Sized + fmt::Debug> fmt::Debug for Grc<C, T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		<T as fmt::Debug>::fmt(&**self, f)
	}
}
impl<C: Counter, T> fmt::Pointer for Grc<C, T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		<_ as fmt::Pointer>::fmt(&self.ptr.as_ptr(), f)
	}
}

impl<C: Counter, T: ?Sized> ops::Drop for Grc<C, T>
{
	fn drop(&mut self)
	{
		// SAFE: Correct pointer accesses, only deallocs if this was last reference
		unsafe
		{
			use core::intrinsics::drop_in_place;
			use core::mem::{size_of_val,align_of_val};
			let ptr = self.ptr.as_ptr();
				
			if (*ptr).strong.dec() // && (*ptr).weak.is_zero()
			{
				drop_in_place( &mut (*ptr).val );
				::memory::heap::dealloc_raw(ptr as *mut (), size_of_val(&*ptr), align_of_val(&*ptr));
			}
		}
	}
}

impl<C: Counter, T> GrcInner<C, T>
{
	fn new_ptr(value: T) -> *mut GrcInner<C, T>
	{
		// SAFE: Correct call to alloc (TODO: Why is alloc unsafe?)
		unsafe {
			::memory::heap::alloc( GrcInner {
				strong: C::one(),
				//weak: C::zero(),
				val: value,
				} )
		}
	}
}

impl<C: Counter, U> Grc<C, [U]>
{
	fn rcinner_align() -> usize {
		::core::cmp::max(::core::mem::align_of::<U>(), ::core::mem::align_of::<usize>())
	}
	unsafe fn rcinner_ptr(count: usize, ptr: *mut ()) -> *mut GrcInner<C, [U]> {
		::core::mem::transmute(::core::slice::from_raw_parts(ptr, count))
	}
	fn rcinner_size(len: usize) -> usize {
		// SAFE: (TODO: Check validity here) Should not cause a read from invalid pointer
		unsafe {
			let ptr = Self::rcinner_ptr(len, 1 as *mut ());
			::core::mem::size_of_val(&*ptr)
		}
	}
	
	/// Construct an unsized allocation from a size and populating method
	pub fn from_fn<F>(len: usize, mut fcn: F) -> Self
	where
		F: FnMut(usize)->U
	{
		let align = Self::rcinner_align();
		let size = Self::rcinner_size(len);
		
		// SAFE: No mut aliasing, no read from undefined, hopefull correct behavior
		unsafe {
			let ptr = ::memory::heap::alloc_raw(size, align);
			let inner = Self::rcinner_ptr(len, ptr);
			::core::ptr::write( &mut (*inner).strong, C::one() );
			for i in 0 .. len {
				::core::ptr::write( (*inner).val.as_mut_ptr().offset(i as isize), fcn(i) );
			}
			
			Grc { ptr: NonNull::new_unchecked(inner) }
		}
	}
	
	/// Construct an unsized allocation using an exactly-sized iterator
	pub fn from_iter<T>(iterator: T) -> Self
	where
		T: IntoIterator<Item=U>,
		T::IntoIter: ExactSizeIterator,
	{
		let mut it = iterator.into_iter();
		let len = it.len();
		
		Self::from_fn(len, |_| it.next().expect("ExactSizeIterator violated in Grc::from_iter"))
	}
}

