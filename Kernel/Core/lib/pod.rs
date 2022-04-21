// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/pod.rs
//! Plain-old-data support

/// Plain-old-data trait
pub unsafe auto trait POD {}

//impl<T: ::core::ops::Drop> !POD for T {}  // - I would love this, but it collides with every other !POD impl
impl<T> !POD for ::core::cell::UnsafeCell<T> {}
impl<T> !POD for ::core::ptr::NonNull<T> {}
impl<T> !POD for crate::lib::mem::boxed::Box<T> {}
impl<T> !POD for *const T {}
impl<T> !POD for *mut T {}
impl<'a, T> !POD for &'a T {}
impl<'a, T> !POD for &'a mut T {}

// TODO: Can there be an impl for the atomics?
unsafe impl POD for ::core::sync::atomic::AtomicU32 {}
#[cfg(target_has_atomic="64")]
unsafe impl POD for ::core::sync::atomic::AtomicU64 {}

pub fn as_byte_slice<T: ?Sized + POD>(s: &T) -> &[u8] {
	// SAFE: Plain-old-data
	unsafe { ::core::slice::from_raw_parts(s as *const _ as *const u8, ::core::mem::size_of_val(s)) }
}
pub fn as_byte_slice_mut<T: ?Sized + POD>(s: &mut T) -> &mut [u8] {
	// SAFE: Plain-old-data
	unsafe { ::core::slice::from_raw_parts_mut(s as *mut _ as *mut u8, ::core::mem::size_of_val(s)) }
}

pub trait PodHelpers
{
	fn zeroed() -> Self where Self: Sized + POD {
		// SAFE: This method is only ever valid when Self: POD, which allows any bit pattern
		unsafe { ::core::mem::zeroed() }
	}
	fn as_byte_slice(&self) -> &[u8];
	fn as_byte_slice_mut(&mut self) -> &mut [u8];
}
impl<T: ?Sized + POD> PodHelpers for T {
	fn as_byte_slice(&self) -> &[u8] {
		as_byte_slice(self)
	}
	fn as_byte_slice_mut(&mut self) -> &mut [u8] {
		as_byte_slice_mut(self)
	}
}
