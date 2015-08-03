// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/lazy_static.rs
/// 
use prelude::*;

/// A lazily initialised value (for `static`s)
pub struct LazyStatic<T: Send+Sync>(pub ::core::cell::UnsafeCell<Option<T>>);
unsafe impl<T: Send+Sync> Sync for LazyStatic<T> {}	// Barring the unsafe "prep" call, is Sync
unsafe impl<T: Send+Sync> Send for LazyStatic<T> {}	// Sendable because inner is sendable

#[macro_export]
macro_rules! lazystatic_init {
	() => ( $crate::lib::LazyStatic::new() );
}

impl<T: Send+Sync> LazyStatic<T>
{
	pub const fn new() -> Self {
		LazyStatic ( ::core::cell::UnsafeCell::new(None) )
	}
	
	/// (unsafe) Prepare the value using the passed function
	///
	/// Unsafe because it must NOT be called where anything else is accessing the LazyStatic.
	pub unsafe fn prep<Fcn: FnOnce()->T>(&self, fcn: Fcn) {
		let r = &mut *self.0.get();
		assert!(r.is_none(), "LazyStatic<{}> initialised multiple times", type_name!(T));
		if r.is_none() {
			*r = Some(fcn());
		}
	}
	/// Returns true if the static has been initialised
	pub fn ls_is_valid(&self) -> bool {
		// SAFE: No aliasing possible unless 'prep' is called in a racy manner
		unsafe {
			(*self.0.get()).is_some()
		}
	}
	/// (unsafe) Obtain a mutable reference to the interior
	pub unsafe fn ls_unsafe_mut(&self) -> &mut T {
		match *self.0.get()
		{
		Some(ref mut v) => v,
		None => panic!("Dereferencing LazyStatic<{}> without initialising", type_name!(T))
		}
	}
}
impl<T: Send+Sync> ::core::ops::Deref for LazyStatic<T>
{
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: No aliasing possible unless 'prep' is called in a racy manner
		match unsafe { (&*self.0.get()).as_ref() } {
		Some(v) => v,
		None => panic!("Dereferencing LazyStatic<{}> without initialising", type_name!(T))
		}
	}
}

