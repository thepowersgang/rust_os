// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/opt_ptr.rs
//! 

/// An equivalemnt of Option<*const T> which cannot be NULL
pub struct OptPtr<T>(pub *const T);
unsafe impl<T: Send> Send for OptPtr<T> {}
/// An equivalemnt of Option<*mut T> which cannot be NULL
pub struct OptMutPtr<T>(pub *mut T);
unsafe impl<T: Send> Send for OptMutPtr<T> {}

impl<T> OptPtr<T>
{
	pub fn is_none(&self) -> bool {
		self.0.is_null()
	}
	pub fn is_some(&self) -> bool {
		!self.0.is_null()
	}
	pub fn unwrap(&self) -> *const T {
		assert!( !self.0.is_null() );
		self.0
	}
	pub unsafe fn as_ref(&self) -> Option<&T> {
		if (self.0).is_null() {
			None
		}
		else {
			Some(&*self.0)
		}
	}
	pub unsafe fn as_mut(&self) -> OptMutPtr<T> {
		::core::mem::transmute(self)
	}
	/// This is HIGHLY unsafe
	pub unsafe fn as_mut_ref(&self) -> Option<&mut T> {
		::core::mem::transmute(self.as_ref())
	}
}

impl<T> OptMutPtr<T>
{
	pub fn is_none(&self) -> bool {
		self.0.is_null()
	}
	pub fn is_some(&self) -> bool {
		!self.0.is_null()
	}
	pub fn unwrap(&self) -> *mut T {
		assert!( !self.0.is_null() );
		self.0
	}
	pub unsafe fn as_ref(&self) -> Option<&mut T> {
		if (self.0).is_null() {
			None
		}
		else {
			Some(&mut *self.0)
		}
	}
}

