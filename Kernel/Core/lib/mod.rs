//
//
//
#![macro_escape]
use core::option::{Option,Some,None};
use core::ptr::RawPtr;

pub use self::queue::Queue;
pub use self::vec::Vec;
pub use self::string::String;

pub mod clone;

pub mod mem;
pub mod queue;
pub mod vec;
pub mod string;

pub mod num
{
	pub fn round_up(val: uint, target: uint) -> uint
	{
		return (val + target-1) / target * target;
	}
}

pub mod collections
{
	pub trait MutableSeq<T>
	{
		fn push(&mut self, t: T);
		fn pop(&mut self) -> ::core::option::Option<T>;
	}
}

pub struct OptPtr<T>(pub *const T);
pub struct OptMutPtr<T>(pub *mut T);

impl<T> OptPtr<T>
{
	fn is_none(&self) -> bool {
		self.0.is_null()
	}
	fn is_some(&self) -> bool {
		!self.0.is_null()
	}
	fn unwrap(&self) -> *const T {
		assert!( !self.0.is_null() );
		self.0
	}
	unsafe fn as_ref(&self) -> Option<&T> {
		if (self.0).is_null() {
			None
		}
		else {
			Some(&*self.0)
		}
	}
}

impl<T> OptMutPtr<T>
{
	fn is_none(&self) -> bool {
		self.0.is_null()
	}
	fn is_some(&self) -> bool {
		!self.0.is_null()
	}
	fn unwrap(&self) -> *mut T {
		assert!( !self.0.is_null() );
		self.0
	}
	unsafe fn as_ref(&self) -> Option<&mut T> {
		if (self.0).is_null() {
			None
		}
		else {
			Some(&mut *self.0)
		}
	}
}

#[macro_export]
macro_rules! tern(
	($cnd:expr ? $ok:expr : $nok:expr) => (if $cnd { $ok } else { $nok })//,
//	($cnd:expr ? $ok:expr : $($cnd2:expr ? $val2:tt :)* $false:expr ) => (if $cnd { $ok } $(else if $cnd2 { $val2 })* else { $false })
	)

// vim: ft=rust

