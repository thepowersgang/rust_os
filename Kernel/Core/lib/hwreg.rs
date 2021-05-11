use ::core::marker::PhantomData;
use ::core::ops;
use crate::lib::pod::POD;

pub trait Access
{
}

/// Reading has no side-effects, can't write
pub struct PureRead; impl Access for PureRead {}
/// Reading has no side-effects, write allowed
pub struct PureRW; impl Access for PureRW {}
/// Only allows writing (unsafe)
pub struct WriteOnly; impl Access for WriteOnly {}
/// Reading has a side-effect, but can't write
pub struct UnsafeRead; impl Access for UnsafeRead {}
/// Read/write both have sideeffects
pub struct ReadWrite; impl Access for ReadWrite {}

pub struct Reg<'a, T: POD, A: Access>(*const T, PhantomData<&'a T>, PhantomData<A>);

impl<'a, T: POD, A: Access> Reg<'a, T, A>
{
	/// UNSAFETY: Caller must ensure that access is correct, and that pointer is valid for `'a`
	pub unsafe fn from_ptr(p: *const T) -> Self {
		Reg(p, PhantomData, PhantomData)
	}
}

macro_rules! add_method {
	($T:ident [$($trait:path),*] $i:item) => {
		$(
		impl<'a, $T: POD> Reg<'a, $T, $trait>
		{
			$i
		}
		)*
	}
}

add_method!{T [PureRead, PureRW]
	pub fn read(&self) -> T {
		// SAFE: Contract from constructor
		unsafe { ::core::ptr::read_volatile(self.0) }
	}
}
add_method!{T [UnsafeRead, ReadWrite]
	pub unsafe fn read(&self) -> T {
		::core::ptr::read_volatile(self.0)
	}
}

impl<'a, T: POD + ops::BitOr + Copy> Reg<'a, T, PureRW>
{
	pub unsafe fn fetch_or(&self, v: T) -> T {
		let rv = ::core::ptr::read_volatile(self.0);
		::core::ptr::write_volatile(self.0 as *mut _, rv | v);
		rv
	}
}

