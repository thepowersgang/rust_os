
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
}

impl<T> ::core::ops::Deref<T> for Rc<T>
{
	fn deref<'s>(&'s self) -> &'s T
	{
		unsafe { &(*self.inner).val }
	}
}

impl<T> RcInner<T>
{
	unsafe fn new_ptr(value: T) -> *mut RcInner<T>
	{
		let ptr = ::memory::heap::alloc::<RcInner<T>>();
		let tmp = &mut *ptr;
		tmp.count = 1;
		tmp.val = value;
		return ptr;
	}
}

// vim: ft=rust

