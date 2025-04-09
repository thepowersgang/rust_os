// Tifflin OS - System Calls
// - By John Hodge (thePowersGang)
//
//! Provides wrappers around most system calls
//#![feature(core_intrinsics)]
#![feature(thread_local)]
#![feature(stmt_expr_attributes)]
#![feature(generic_const_exprs)]
#![allow(incomplete_features)]	// for `generic_const_exprs`
#![no_std]

pub extern crate syscall_values as values;

mod std {
	//pub use core::convert;
	pub use core::fmt;
}

macro_rules! define_waits {
	($name:ident => ($($n:ident : $n2:ident = $val:expr,)*)) => {
		#[derive(Default)]
		pub struct $name(u32);
		impl ::Waits for $name {
			fn from_val(v: u32) -> $name { $name(v) }
			fn into_val(self) -> u32 { self.0 }
		}
		impl $name
		{
			pub fn new() -> $name { $name(0) }
			$(
			pub fn $n(self) -> $name { $name( self.0 | $val ) }
			pub fn $n2(&self) -> bool { (self.0 & $val) != 0 }
			)*
		}
	};
}

pub enum Void {}

#[cfg(arch="native")]
#[path="raw-native.rs"]
pub mod raw;

#[cfg(not(arch="native"))]
#[cfg_attr(target_arch="x86_64", path="raw-amd64.rs")]
#[cfg_attr(target_arch="arm", path="raw-armv7.rs")]
#[cfg_attr(target_arch="aarch64", path="raw-armv8.rs")]
#[cfg_attr(target_arch="riscv64", path="raw-riscv64.rs")]
mod raw;

mod int_args;

/// Architecture's page size (minimum allocation granularity)
pub const PAGE_SIZE: usize = self::raw::PAGE_SIZE;

#[macro_use]
pub mod logging;

pub mod kcore;
pub mod vfs;
pub mod gui;
pub mod memory;
pub mod threads;
pub mod sync;
pub mod ipc;
pub mod net;

pub use values::WaitItem;

unsafe fn syscall<C: values::Args>(c: C) -> u64
where
	C::Tuple: int_args::CallTuple,
{
	int_args::CallTuple::call(c.into_tuple(), C::CALL)
}

#[doc(hidden)]
pub struct ObjectHandle(u32);
impl ObjectHandle
{
	#[inline]
	fn new(rv: usize) -> Result<ObjectHandle,u32> {
		to_result(rv).map( |v| ObjectHandle(v) )
	}
	#[inline]
	fn into_raw(self) -> u32 {
		let rv = self.0;
		::core::mem::forget(self);
		rv
	}
	#[inline]
	fn call_value(&self, call: u16) -> u32 {
		(1 << 31) | self.0 | (call as u32) << 20
	}
	
	#[inline]
	fn get_wait(&self, mask: u32) -> ::values::WaitItem {
		::values::WaitItem {
			object: self.0,
			flags: mask,
		}
	}

	fn get_class(&self) -> Result<u16,()> {
		// SAFE: Known method
		let v = unsafe { ::raw::syscall_0( self.call_value(::values::OBJECT_GETCLASS) ) };
		if v >= (1<<16) {
			Err( () )
		}
		else {
			Ok( v as u16 )
		}
	}
	fn try_clone(&self) -> Result<Self,()> {
		// SAFE: Standard method
		let v = unsafe { ::raw::syscall_0( self.call_value(::values::OBJECT_CLONE) ) };
		if v >= (1<<20) {
			Err( () )
		}
		else {
			Ok( ObjectHandle(v as u32) )
		}
	}

	unsafe fn call_m<C: values::Args>(&self, c: C) -> u64
	where
		C::Tuple: int_args::CallTuple,
	{
		int_args::CallTuple::call(c.into_tuple(), self.call_value(C::CALL as u16))
	}
	unsafe fn call_v<C: values::Args>(self, c: C) -> u64
	where
		C::Tuple: int_args::CallTuple,
	{
		let id = self.call_value(C::CALL as u16);
		::core::mem::forget(self);	// The kernel handles dropping the object on by-value calls
		int_args::CallTuple::call(c.into_tuple(), id)
	}
}
impl Drop for ObjectHandle {
	fn drop(&mut self) {
		// SAFE: Valid syscall
		unsafe {
			::raw::syscall_0( self.call_value(::values::OBJECT_DROP) );
		}
	}
}


/// Opaque representation of an arbitrary syscall object
pub struct AnyObject(::ObjectHandle);
impl AnyObject
{
	pub fn downcast<T: ::Object>(self) -> Result<T, Self> {
		if self.0.get_class() == Ok(T::CLASS) {
			Ok( T::from_handle(self.0) )
		}
		else {
			Err(self)
		}
	}
	/// Cast this to the specified type, panicking on failure
	pub fn downcast_panic<T: ::Object>(self) -> T {
		match self.0.get_class()
		{
		Ok(c) if c == T::CLASS => T::from_handle(self.0),
		Ok(class) => panic!("AnyObject({})::downcast_panic<{}> - Fail, {} {} != {}",
			self.0 .0, ::core::any::type_name::<T>(),
			class, values::get_class_name(class), T::CLASS
			),
		Err(_) => panic!("AnyObject({})::downcast_panic<{}> - Invalid object num", self.0 .0, ::core::any::type_name::<T>()),
		}
	}
}

#[doc(hidden)]
pub trait Waits: Default {
	fn from_val(v: u32) -> Self;
	fn into_val(self) -> u32;
}
impl Waits for () {
	fn from_val(_: u32) -> () { () }
	fn into_val(self) -> u32 { 0 }
}

/// Trait that provides common methods for syscall objects
pub trait Object
{
	const CLASS: u16;
	fn class() -> u16;
	// TODO: Make this unsafe (and/or hide from the user)
	fn from_handle(handle: ::ObjectHandle) -> Self;
	fn into_handle(self) -> ::ObjectHandle;
	fn handle(&self) -> &::ObjectHandle;

	type Waits: Waits;
	fn get_wait(&self, w: Self::Waits) -> ::values::WaitItem {
		self.handle().get_wait(w.into_val())
	}
	fn check_wait(&self, wi: &::values::WaitItem) -> Self::Waits {
		assert_eq!(wi.object, self.handle().0);
		Self::Waits::from_val(wi.flags)
	}

	fn from_raw(handle: u32) -> Result<Self, FromRawError> where Self: Sized {
		object_from_raw(handle)
	}
}

#[derive(Debug)]
pub enum FromRawError
{
	/// The handle index passed wasn't valid
	BadIndex,
	/// The object at this index wasn't the desired class
	BadClass(u16),
}
impl ::core::fmt::Display for FromRawError
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		match self
		{
		&FromRawError::BadIndex => f.write_str("Invalid object index"),
		&FromRawError::BadClass(v) => write!(f, "Incorrect class: was {}", v),
		}
	}
}

/// Obtain an object handle from a raw handle index
///
/// NOTE: This method is only meant for loader/init to use
pub fn object_from_raw<T: Object>(handle: u32) -> Result<T,::FromRawError> {
	let h = ::ObjectHandle(handle);
	match h.get_class()
	{
	Ok(v) => if v == T::class() {
			Ok(T::from_handle(h))
		}
		else {
			Err(::FromRawError::BadClass(v))
		},
	Err(_) => Err(::FromRawError::BadIndex),
	}
}

#[inline]
fn to_result(val: usize) -> Result<u32,u32> {
	const SIGNAL_VAL: usize = 1 << 31;
	if val < SIGNAL_VAL {
		Ok(val as u32)
	}
	else {
		Err( (val - SIGNAL_VAL) as u32 )
	}
}

pub use values::TextInfo;
pub use self::kcore::get_text_info;
pub use self::kcore::{log_write,debug_value};
pub use self::kcore::system_ticks;



