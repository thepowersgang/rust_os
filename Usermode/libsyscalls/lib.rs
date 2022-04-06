// Tifflin OS - System Calls
// - By John Hodge (thePowersGang)
//
//! Provides wrappers around most system calls
#![feature(core_intrinsics)]
#![feature(thread_local)]
#![feature(stmt_expr_attributes)]
#![cfg_attr(arch="native",feature(rustc_private))]
#![no_std]

mod std {
	pub use core::convert;
	pub use core::fmt;
}

macro_rules! type_name {
	($t:ty) => {::core::intrinsics::type_name::<$t>()};
}

macro_rules! syscall {
	($id:ident) => {
		::raw::syscall_0(::values::$id)
		};
	($id:ident, $arg1:expr) => {
		::raw::syscall_1(::values::$id, $arg1)
		};
	($id:ident, $arg1:expr, $arg2:expr) => {
		::raw::syscall_2(::values::$id, $arg1, $arg2)
		};
	($id:ident, $arg1:expr, $arg2:expr, $arg3:expr) => {
		::raw::syscall_3(::values::$id, $arg1, $arg2, $arg3)
		};
	($id:ident, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr) => {
		::raw::syscall_4(::values::$id, $arg1, $arg2, $arg3, $arg4)
		};
	($id:ident, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr) => {
		::raw::syscall_5(::values::$id, $arg1, $arg2, $arg3, $arg4, $arg5)
		};
	($id:ident, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr, $arg6:expr) => {
		::raw::syscall_6(::values::$id, $arg1, $arg2, $arg3, $arg4, $arg5, $arg6)
		};
}
//macro_rules! slice_arg { ($slice:ident) => { $slice.as_ptr(), $slice.len() } }

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

// File in the root of the repo
#[cfg(not(arch="native"))]
#[path="../../syscalls.inc.rs"]
mod values;

#[cfg(arch="native")]
#[path="../../syscalls.inc.rs"]
pub mod values;

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

/// Archtecture's page size (minimum allocation granuality)
pub const PAGE_SIZE: usize = self::raw::PAGE_SIZE;

#[macro_use]
pub mod logging;

pub mod vfs;
pub mod gui;
pub mod memory;
pub mod threads;
pub mod sync;
pub mod ipc;
pub mod net;

pub use values::WaitItem;

macro_rules! def_call {
	($name:ident,$name_v:ident => $fcn:ident( $($arg_name:ident),* )) => {
		#[allow(dead_code)]
		#[inline(always)]
		unsafe fn $name(&self, call: u16 $(, $arg_name: usize)*) -> u64 {
			assert!(call < 0x400);
			::raw::$fcn( self.call_value(call) $(, $arg_name)* )
		}
		#[allow(dead_code)]
		#[inline(always)]
		unsafe fn $name_v(self, call: u16 $(, $arg_name: usize)*) -> u64 {
			assert!(call >= 0x400);
			let cv = self.call_value(call);
			::core::mem::forget(self);
			::raw::$fcn( cv $(, $arg_name)* )
		}
	}
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

	def_call!{ call_0,call_0_v => syscall_0() }
	def_call!{ call_1,call_1_v => syscall_1(a1) }
	def_call!{ call_2,call_2_v => syscall_2(a1, a2) }
	def_call!{ call_3,call_3_v => syscall_3(a1, a2, a3) }
	def_call!{ call_4,call_4_v => syscall_4(a1, a2, a3, a4) }
	def_call!{ call_5,call_5_v => syscall_5(a1, a2, a3, a4, a5) }
	def_call!{ call_6,call_6_v => syscall_6(a1, a2, a3, a4, a5, a6) }

	#[allow(dead_code)]
	#[inline]
	unsafe fn call_2l(&self, call: u16, a1: u64, a2: usize) -> u64 {
		#[cfg(target_pointer_width="64")]
		{ return ::raw::syscall_2( self.call_value(call), a1 as usize, a2 ) }
		#[cfg(target_pointer_width="32")]
		{ return ::raw::syscall_3( self.call_value(call), (a1 & 0xFFFFFFFF) as usize, (a1 >> 32) as usize, a2 ) }
	}
	#[allow(dead_code)]
	#[inline]
	unsafe fn call_3l(&self, call: u16, a1: u64, a2: usize, a3: usize) -> u64 {
		#[cfg(target_pointer_width="64")]
		{ return ::raw::syscall_3( self.call_value(call), a1 as usize, a2, a3 ) }
		#[cfg(target_pointer_width="32")]
		{ return ::raw::syscall_4( self.call_value(call), (a1 & 0xFFFFFFFF) as usize, (a1 >> 32) as usize, a2, a3 ) }
	}

	#[allow(dead_code)]
	#[inline]
	unsafe fn call_4l(&self, call: u16, a1: u64, a2: usize, a3: usize, a4: usize) -> u64 {
		#[cfg(target_pointer_width="64")]
		return ::raw::syscall_4( self.call_value(call), a1 as usize, a2, a3, a4 );
		#[cfg(target_pointer_width="32")]
		return ::raw::syscall_5( self.call_value(call), (a1 & 0xFFFFFFFF) as usize, (a1 >> 32) as usize, a2, a3, a4 );
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

/// Opaque representation of an arbitary syscall object
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
	/// Cast this to the specified type, panicing on failure
	pub fn downcast_panic<T: ::Object>(self) -> T {
		match self.0.get_class()
		{
		Ok(c) if c == T::CLASS => T::from_handle(self.0),
		Ok(class) => panic!("AnyObject({})::downcast_panic<{}> - Fail, {} {} != {}",
			self.0 .0, type_name!(T),
			class, values::get_class_name(class), T::CLASS
			),
		Err(_) => panic!("AnyObject({})::downcast_panic<{}> - Invalid object num", self.0 .0, type_name!(T)),
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

#[inline]
/// Write a string to the kernel's log
pub fn log_write<S: ?Sized+AsRef<[u8]>>(msg: &S) {
	let msg = msg.as_ref();
	// SAFE: Syscall
	unsafe { syscall!(CORE_LOGWRITE, msg.as_ptr() as usize, msg.len()); }
}
pub fn debug_value<S: ?Sized+AsRef<[u8]>>(msg: &S, v: usize) {
	let msg = msg.as_ref();
	// SAFE: Syscall
	unsafe { syscall!(CORE_DBGVALUE, msg.as_ptr() as usize, msg.len(), v); }
}


pub use values::TEXTINFO_KERNEL;

#[inline]
/// Obtain a string from the kernel
/// 
/// Accepts a buffer and returns a string slice from that buffer.
pub fn get_text_info(unit: u32, id: u32, buf: &mut [u8]) -> &str {
	// SAFE: Syscall
	let len: usize = unsafe { syscall!(CORE_TEXTINFO, unit as usize, id as usize,  buf.as_ptr() as usize, buf.len()) } as usize;
	::core::str::from_utf8(&buf[..len]).expect("TODO: get_text_info handle error")
}



