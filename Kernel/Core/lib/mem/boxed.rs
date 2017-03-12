// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mem/mod.rs
//! Owned dynamic allocation (box)
use core::{mem,ops,fmt,marker};

#[lang = "owned_box"]
pub struct Box<T: ?Sized>(::core::ptr::Unique<T>);

impl<T: ?Sized + marker::Unsize<U>, U: ?Sized> ops::CoerceUnsized<Box<U>> for Box<T> { }

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

impl<T> ops::Boxed for Box<T> {
	type Data = T;
	type Place = IntermediateBox<T>;
	unsafe fn finalize(b: IntermediateBox<T>) -> Box<T> {
		let p = b.ptr;
		::core::mem::forget(b);
		// TODO: Unsized?
		::core::mem::transmute(p)
	}
}
pub struct IntermediateBox<T: ?Sized> {
	ptr: *mut u8,
	size: usize,
	align: usize,
	marker: marker::PhantomData<*mut T>,
}
impl<T: ?Sized> Drop for IntermediateBox<T> {
	fn drop(&mut self) {
		if self.size > 0 {
			// SAFE: Pointer is valid, no need to drop
			unsafe { ::memory::heap::dealloc_raw(self.ptr as *mut (), self.size, self.align) }
		}
	}
}
impl<T> ops::BoxPlace<T> for IntermediateBox<T> {
	fn make_place() -> IntermediateBox<T> {
		let size = mem::size_of::<T>();
		let align = mem::align_of::<T>();
		
		let ptr = if size == 0 {
				::memory::heap::ZERO_ALLOC as *mut u8
			}
			else {
				// SAFE: Tell me again, why is allo unsafe?
				unsafe { ::memory::heap::alloc_raw(size, align) as *mut u8 }
			};

		IntermediateBox {
			ptr: ptr,
			size: size,
			align: align,
			marker: marker::PhantomData,
			}
	}
}
impl<T> ops::Place<T> for IntermediateBox<T> {
	fn pointer(&mut self) -> *mut T {
		self.ptr as *mut T
	}
}

pub fn into_inner<T>(b: Box<T>) -> T {
	let box v = b;
	v
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Box<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		(**self).fmt(f)
	}
}
impl<T: ?Sized + fmt::Display> fmt::Display for Box<T> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		(**self).fmt(f)
	}
}
impl<T: ?Sized> ops::Deref for Box<T> {
	type Target = T;

	fn deref(&self) -> &T {
		&**self
	}
}
impl<T: ?Sized> ops::DerefMut for Box<T> {
	fn deref_mut(&mut self) -> &mut T {
		&mut **self
	}
}

unsafe impl<#[may_dangle] T: ?Sized> ops::Drop for Box<T> {
	fn drop(&mut self) {
	}
}

