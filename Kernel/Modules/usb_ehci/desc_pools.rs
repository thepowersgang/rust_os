//! Descriptor pools
//!
//! ## Implementation details
//! Both of these pools return handles that "own" a set of allocated data
//! - The hardware structures (stored in a hardware accessible page)
//! - and metadata (stored in a separate inline array)
mod qh_pool;
mod td_pool;
pub use self::qh_pool::{QhPool, QhHandle};
pub use self::td_pool::{TdPool, TdHandle};

fn set_first_zero_bit(arr: &mut [u8], start: usize) -> Option<usize> {
	if start > 0 {
		for (i,s) in arr.iter_mut().enumerate() {
			if *s != 0xFF {
				let j = s.trailing_ones() as usize;
				let rv = i * 8 + j;
				if rv >= start {
					*s |= 1 << j;
					return Some(rv);
				}       
			}
		}
	}
	for (i,s) in arr.iter_mut().enumerate() {
		if *s != 0xFF {
			let j = s.trailing_ones() as usize;
			*s |= 1 << j;
			return Some(i * 8 + j);
		}
	}
	None
}
fn get_and_clear_bit(arr: &mut [u8], idx: usize) -> bool {
	let bit = 1 << (idx % 8);
	let s = &mut arr[idx / 8];
	let rv = *s & bit != 0;
	*s &= !bit;
	rv
}

/// Helper - A wrapper around an `AllocHandle` that allows unsafe per-element mutable
struct UnsafeArrayHandle<T> {
	inner: ::kernel::memory::virt::AllocHandle,
	pd: ::core::marker::PhantomData<::core::cell::UnsafeCell<T>>,
}
unsafe impl<T: Sync> Sync for UnsafeArrayHandle<T> {}
unsafe impl<T: Send> Send for UnsafeArrayHandle<T> {}
impl<T: ::kernel::lib::POD> UnsafeArrayHandle<T> {
	fn new(inner: ::kernel::memory::virt::AllocHandle) -> Self {
		Self { inner, pd: ::core::marker::PhantomData }
	}
	fn get_phys(&self, idx: usize) -> ::kernel::memory::PAddr {
		::kernel::memory::virt::get_phys::<T>( self.inner.as_ref(idx * ::core::mem::size_of::<T>()) )
	}
	unsafe fn get_raw(&self, idx: usize) -> *mut T {
		self.inner.as_int_mut(idx * ::core::mem::size_of::<T>())
	}
	unsafe fn get(&self, idx: usize) -> &T {
		self.inner.as_ref(idx * ::core::mem::size_of::<T>())
	}
	unsafe fn get_mut(&self, idx: usize) -> &mut T {
		self.inner.as_int_mut(idx * ::core::mem::size_of::<T>())
	}
}