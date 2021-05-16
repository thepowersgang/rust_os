//
//
//
//! Hardware registers
use ::core::marker::PhantomData;
use ::core::ops;
use crate::lib::pod::POD;

mod sealed { pub trait Sealed {} }
pub trait Access: sealed::Sealed
{
}

macro_rules! def_access {
	($( $(#[$attr:meta])* pub struct $name:ident; )*) => { $( $(#[$attr])* pub struct $name; impl Access for $name {} impl sealed::Sealed for $name {} )* };
}
def_access! {
	/// Reading has no side-effects, can't write
	pub struct AccessPureRead;
	/// Reading has no side-effects, write allowed
	pub struct AccessPureRW;
	pub struct AccessSafeRW;
	pub struct AccessSafeWO;
	/// Only allows writing (unsafe)
	pub struct AccessWriteOnly;
	/// Reading has a side-effect, but can't write
	pub struct AccessUnsafeRead;
	/// Read/write both have sideeffects
	pub struct AccessReadWrite;
}

pub type PureRead  <'a, T> = Reg<'a, T, AccessPureRead>;
pub type PureRW    <'a, T> = Reg<'a, T, AccessPureRW>;
pub type SafeRW    <'a, T> = Reg<'a, T, AccessSafeRW>;
pub type SafeWO    <'a, T> = Reg<'a, T, AccessSafeWO>;
pub type WriteOnly <'a, T> = Reg<'a, T, AccessWriteOnly>;
pub type UnsafeRead<'a, T> = Reg<'a, T, AccessUnsafeRead>;
pub type ReadWrite <'a, T> = Reg<'a, T, AccessReadWrite>;

pub struct Reg<'a, T: POD, A: Access>(*const T, PhantomData<&'a T>, PhantomData<A>);
// NOTE: Not Sync, accesses are uncontrolled
unsafe impl<'a, T: POD + Send, A: Access> Send for Reg<'a, T, A> {}

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

add_method!{T [AccessPureRead, AccessPureRW, AccessSafeRW]
	/// Read from the register (safe annotated)
	pub fn load(&self) -> T {
		// SAFE: Contract from constructor
		unsafe { ::core::ptr::read_volatile(self.0) }
	}
}
add_method!{T [AccessUnsafeRead, AccessReadWrite]
	/// Read from the register (unsafe annotated)
	pub unsafe fn load(&self) -> T {
		::core::ptr::read_volatile(self.0)
	}
}
add_method!{T [AccessWriteOnly, AccessPureRW, AccessReadWrite]
	/// Write to the register (always assumed to be unsafe)
	pub unsafe fn store(&self, v: T) {
		::core::ptr::write_volatile(self.0 as *mut _, v)
	}
}
add_method!{T [AccessSafeRW, AccessSafeWO]
	/// Write to the register (always assumed to be unsafe)
	pub fn store(&self, v: T) {
		// SAFE: Contract from constructor
		unsafe {
			::core::ptr::write_volatile(self.0 as *mut _, v)
		}
	}
}

impl<'a, T: POD + ops::BitOr + Copy> Reg<'a, T, AccessPureRW>
{
	/// Read, bitwise or, write, return
	pub unsafe fn fetch_or(&self, v: T) -> T {
		let rv = ::core::ptr::read_volatile(self.0);
		::core::ptr::write_volatile(self.0 as *mut _, rv | v);
		rv
	}
}
impl<'a, T: POD + ops::BitAnd + Copy> Reg<'a, T, AccessPureRW>
{
	/// Read, bitwise and, write, return
	pub unsafe fn fetch_and(&self, v: T) -> T {
		let rv = ::core::ptr::read_volatile(self.0);
		::core::ptr::write_volatile(self.0 as *mut _, rv & v);
		rv
	}
}

impl<'a, T: POD + ops::BitOr + Copy> Reg<'a, T, AccessSafeRW>
{
	/// Read, bitwise or, write, return
	pub fn fetch_or(&self, v: T) -> T {
		// SAFE: Contract from constructor
		unsafe {
			let rv = ::core::ptr::read_volatile(self.0);
			::core::ptr::write_volatile(self.0 as *mut _, rv | v);
			rv
		}
	}
}
impl<'a, T: POD + ops::BitAnd + Copy> Reg<'a, T, AccessSafeRW>
{
	/// Read, bitwise and, write, return
	pub fn fetch_and(&self, v: T) -> T {
		// SAFE: Contract from constructor
		unsafe {
			let rv = ::core::ptr::read_volatile(self.0);
			::core::ptr::write_volatile(self.0 as *mut _, rv & v);
			rv
		}
	}
}

