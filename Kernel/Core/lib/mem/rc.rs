//
//
//
use _common::*;
use core::atomic::{AtomicUsize,Ordering};

pub struct Rc<T>
{
	inner: *mut RcInner<T>,
}

impl<T> !Send for Rc<T> {}
unsafe impl<T: Sync> Sync for Rc<T> {}

struct RcInner<T>
{
	count: usize,
	val: T,
}

pub struct Arc<T>
{
	inner: *const ArcInner<T>,
}
unsafe impl<T: Send> Send for Arc<T> {}
unsafe impl<T: Sync> Sync for Arc<T> {}

struct ArcInner<T>
{
	count: AtomicUsize,
	val: T,
}

impl<T> Rc<T>
{
	pub fn new(value: T) -> Rc<T>
	{
		unsafe {
			Rc {
				inner: RcInner::new_ptr(value)
			}
		}
	}
	pub fn is_same(&self, other: &Rc<T>) -> bool {
		self.inner == other.inner
	}
}

impl<T> Clone for Rc<T>
{
	fn clone(&self) -> Rc<T>
	{
		unsafe { (*self.inner).count += 1; }
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
		unsafe { &(*self.inner).val }
	}
}

#[unsafe_destructor]
impl<T> ::core::ops::Drop for Rc<T>
{
	fn drop(&mut self)
	{
		unsafe
		{
			(*self.inner).count -= 1;
			if (*self.inner).count == 0
			{
				drop( ::core::ptr::read( &(*self.inner).val ) );
				::memory::heap::dealloc(self.inner);
			}
			self.inner = 0 as *mut _;
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
	pub fn new(value: T) -> Arc<T>
	{
		Arc {
			inner: unsafe { ::memory::heap::alloc( ArcInner {
				count: AtomicUsize::new(1),
				val: value
				} ) },
		}
	}
}
impl<T> Clone for Arc<T>
{
	fn clone(&self) -> Arc<T>
	{
		unsafe {
			(*self.inner).count.fetch_add(1, Ordering::Acquire);
		}
		Arc {
			inner: self.inner
		}
	}
}
#[unsafe_destructor]
impl<T> ::core::ops::Drop for Arc<T>
{
	fn drop(&mut self)
	{
		unsafe
		{
			let oldcount = (*self.inner).count.fetch_sub(1, Ordering::Release);
			if oldcount == 1
			{
				drop( ::core::ptr::read( &(*self.inner).val ) );
				::memory::heap::dealloc(self.inner as *mut ArcInner<T>);
			}
		}
		self.inner = 0 as *const _;
	}
}

impl<T> ::core::ops::Deref for Arc<T>
{
	type Target = T;
	fn deref<'s>(&'s self) -> &'s T
	{
		unsafe { &(*self.inner).val }
	}
}

// vim: ft=rust

