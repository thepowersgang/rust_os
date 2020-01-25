//! Handles that manage their own deallocation
use core::ops;

pub trait RemoteBuffer//: RemoteFree
{
	fn get(&self) -> &[u8];
}

pub trait RemoteFree
{
	/// Deallocate the backing memory for `self`
	/// This _must_ be called through an owned pointer, and the pointer must be discarded after the call.
	unsafe fn free_self(&mut self);
}

/// An owned pointer to a type that handles freeing itself
pub struct Handle<T: ?Sized + RemoteFree>(::core::ptr::NonNull<T>);
impl<T: ?Sized+RemoteFree> Handle<T>
{
	pub unsafe fn new(ptr: *mut T) -> Handle<T> {
		Handle(::core::ptr::NonNull::new_unchecked(ptr))
	}
}
impl<T: ?Sized+RemoteFree> ops::Deref for Handle<T> {
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: Owned pointer
		unsafe { self.0.as_ref() }
	}
}
impl<T: ?Sized+RemoteFree> Drop for Handle<T> {
	fn drop(&mut self) {
		// SAFE: This is unsafe to construct
		unsafe { self.0.as_mut().free_self(); }
	}
}
