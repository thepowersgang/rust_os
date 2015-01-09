//
//
//
use _common::{Option,Some,None};
use core::ptr::PtrExt;
use core::kinds::Send;
use core::ops::Fn;
use lib::mem::Box;

pub use self::queue::Queue;
pub use self::vec::Vec;
pub use self::string::String;

pub mod clone;

pub mod mem;
#[macro_use]
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

pub struct LazyStatic<T>(pub Option<Box<T>>);

impl<T> LazyStatic<T>
{
	pub fn prep<Fcn: Fn()->T>(&mut self, fcn: Fcn) {
		if self.0.is_none() {
			self.0 = Some(box fcn());
		}
	}
}
impl<T> ::core::ops::Deref for LazyStatic<T>
{
	type Target = T;
	fn deref(&self) -> &T {
		&**self.0.as_ref().unwrap()
	}
}
impl<T> ::core::ops::DerefMut for LazyStatic<T>
{
	fn deref_mut(&mut self) -> &mut T {
		&mut **self.0.as_mut().unwrap()
	}
}

pub struct OptPtr<T>(pub *const T);
unsafe impl<T: Send> Send for OptPtr<T> {}
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

pub trait UintBits
{
	fn bit(&self, idx: uint) -> Self;
	fn bits(&self, idx: uint, idx2: uint) -> Self;
}

impl UintBits for u16 {
	fn bit(&self, idx: uint) -> u16 { (*self >> idx) & 1 }
	fn bits(&self, idx: uint, idx2: uint) -> u16 {
		(*self >> idx) & ((1 << (idx2 - idx))-1)
	}
}

#[macro_export]
macro_rules! tern{
	($cnd:expr ? $ok:expr : $nok:expr) => (if $cnd { $ok } else { $nok });
	}

// vim: ft=rust

