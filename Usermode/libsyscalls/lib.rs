// Tifflin OS - System Calls
// - By John Hodge (thePowersGang)
//
// Provides wrappers around most system calls
#![feature(no_std,core_str_ext,core_slice_ext)]
#![feature(core_intrinsics)]
#![feature(asm)]
#![feature(thread_local,const_fn)]
#![feature(associated_consts)]
#![no_std]

mod std {
	pub use core::convert;
	pub use core::fmt;
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
#[path="../../syscalls.inc.rs"]
mod values;

pub enum Void {}

#[cfg(arch="amd64")] #[path="raw-amd64.rs"]
mod raw;
#[cfg(arch="armv7")] #[path="raw-armv7.rs"]
mod raw;

#[macro_use]
pub mod logging;

pub mod vfs;
pub mod gui;
pub mod memory;
pub mod threads;
pub mod sync;

pub use values::WaitItem;

#[doc(hidden)]
pub struct ObjectHandle(u32);
impl ObjectHandle
{
	fn new(rv: usize) -> Result<ObjectHandle,u32> {
		to_result(rv).map( |v| ObjectHandle(v) )
	}
	fn into_raw(self) -> u32 {
		let rv = self.0;
		::core::mem::forget(self);
		rv
	}
	fn call_value(&self, call: u16) -> u32 {
		(1 << 31 | self.0 | (call as u32) << 20)
	}
	
	fn get_wait(&self, mask: u32) -> ::values::WaitItem {
		::values::WaitItem {
			object: self.0,
			flags: mask,
		}
	}
	
	#[allow(dead_code)]
	unsafe fn call_0(&self, call: u16) -> u64 {
		::raw::syscall_0( self.call_value(call) )
	}
	#[allow(dead_code)]
	unsafe fn call_1(&self, call: u16, a1: usize) -> u64 {
		::raw::syscall_1( self.call_value(call), a1)
	}
	#[allow(dead_code)]
	unsafe fn call_2(&self, call: u16, a1: usize, a2: usize) -> u64 {
		::raw::syscall_2( self.call_value(call), a1, a2 )
	}

	#[allow(dead_code)]
	unsafe fn call_3(&self, call: u16, a1: usize, a2: usize, a3: usize) -> u64 {
		::raw::syscall_3( self.call_value(call), a1, a2, a3 )
	}
	#[cfg(target_pointer_width="64")]
	#[allow(dead_code)]
	unsafe fn call_3l(&self, call: u16, a1: u64, a2: usize, a3: usize) -> u64 {
		::raw::syscall_3( self.call_value(call), a1 as usize, a2, a3 )
	}
	#[cfg(target_pointer_width="32")]
	#[allow(dead_code)]
	unsafe fn call_3l(&self, call: u16, a1: u64, a2: usize, a3: usize) -> u64 {
		::raw::syscall_4( self.call_value(call), (a1 & 0xFFFFFFFF) as usize, (a1 >> 32) as usize, a2, a3 )
	}

	#[allow(dead_code)]
	unsafe fn call_4(&self, call: u16, a1: usize, a2: usize, a3: usize, a4: usize) -> u64 {
		::raw::syscall_4( self.call_value(call), a1, a2, a3, a4 )
	}
	#[cfg(target_pointer_width="64")]
	#[allow(dead_code)]
	unsafe fn call_4l(&self, call: u16, a1: u64, a2: usize, a3: usize, a4: usize) -> u64 {
		::raw::syscall_4( self.call_value(call), a1 as usize, a2, a3, a4 )
	}
	#[cfg(target_pointer_width="32")]
	#[allow(dead_code)]
	unsafe fn call_4l(&self, call: u16, a1: u64, a2: usize, a3: usize, a4: usize) -> u64 {
		::raw::syscall_5( self.call_value(call), (a1 & 0xFFFFFFFF) as usize, (a1 >> 32) as usize, a2, a3, a4 )
	}

	#[allow(dead_code)]
	unsafe fn call_5(&self, call: u16, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize) -> u64 {
		::raw::syscall_5( self.call_value(call), a1, a2, a3, a4, a5 )
	}
	#[allow(dead_code)]
	unsafe fn call_6(&self, call: u16, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize) -> u64 {
		::raw::syscall_6( self.call_value(call), a1, a2, a3, a4, a5, a6 )
	}
}
impl Drop for ObjectHandle {
	fn drop(&mut self) {
		// SAFE: Valid call
		unsafe {
			::raw::syscall_0( (1 << 31 | (0x7FF << 20) | self.0) );
		}
	}
}

trait Waits: Default {
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
	fn from_handle(handle: ObjectHandle) -> Self;
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
}

fn to_result(val: usize) -> Result<u32,u32> {
	const SIGNAL_VAL: usize = (1 << 31);
	if val < SIGNAL_VAL {
		Ok(val as u32)
	}
	else {
		Err( (val - SIGNAL_VAL) as u32 )
	}
}

#[inline]
/// Write a string to the kernel's log
pub fn log_write(msg: &str) {
	// SAFE: Syscall
	unsafe { syscall!(CORE_LOGWRITE, msg.as_ptr() as usize, msg.len()); }
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



