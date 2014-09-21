//
//
//
use _common::*;

pub struct Rc<T>
{
	inner: *mut RcInner<T>,
}

struct RcInner<T>
{
	count: uint,
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

//impl<T> PartialEq for Rc<T>
//{
//	fn eq(&self, other: &Rc<T>) -> bool
//	{
//		return self.inner == other.inner;
//	}
//}

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

impl<T> ::core::ops::Deref<T> for Rc<T>
{
	fn deref<'s>(&'s self) -> &'s T
	{
		unsafe { &(*self.inner).val }
	}
}

#[cfg(nonstupid_rust)]
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
				(*self.inner).val = ::core::mem::uninitialized();
				::memory::heap::deallocate( self.inner );
			}
			self.inner = RawPtr::null();
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

