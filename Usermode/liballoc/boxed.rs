
use core::ptr::Unique;
use core::marker::Unsize;
use core::ops::CoerceUnsized;

#[lang = "owned_box"]
pub struct Box<T: ?Sized>( Unique<T> );

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Box<U>> for Box<T> { }

impl<T> Box<T>
{
	pub fn new(v: T) -> Box<T> {
		box v
	}
}

impl<T: ?Sized> Box<T>
{
	pub unsafe fn from_raw(v: *mut T) -> Self {
		::core::mem::transmute(v)
	}
	pub fn into_raw(this: Self) -> *mut T {
		// SAFE: Box<T> and *mut T have the same repr
		unsafe {
			::core::mem::transmute(this)
		}
	}
}

pub struct IntermediateBox<T: ?Sized> {
	ptr: *const u8,
	_marker: ::core::marker::PhantomData<*mut T>,
}
impl<T> ::core::ops::BoxPlace<T> for IntermediateBox<T> {
	fn make_place() -> IntermediateBox<T> {
		IntermediateBox {
			ptr: ::heap::allocate(::core::mem::size_of::<T>(), ::core::mem::align_of::<T>()),
			_marker: ::core::marker::PhantomData,
			}
	}
}
impl<T> ::core::ops::Place<T> for IntermediateBox<T> {
	fn pointer(&mut self) -> *mut T {
		self.ptr as *mut T
	}
}
impl<T> ::core::ops::InPlace<T> for IntermediateBox<T> {
	type Owner = Box<T>;
	unsafe fn finalize(self) -> Box<T> {
		let p = self.ptr;
		::core::mem::forget(self);
		::core::mem::transmute(p)
	}
}

