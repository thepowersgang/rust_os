// Tifflin OS - System Calls
// - By John Hodge (thePowersGang)
//
// Provides wrappers around most system calls
#![feature(no_std,core_str_ext,core_slice_ext)]
#![feature(core_intrinsics)]
#![feature(asm)]
#![feature(thread_local,const_fn)]
#![feature(associated_consts)]
#![feature(result_expect)]
#![no_std]

//use core::prelude::*;
//#[macro_use]
//extern crate core;

extern crate std_io;

mod std {
	pub use core::convert;
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
macro_rules! slice_arg { ($slice:ident) => { $slice.as_ptr(), $slice.len() } }

// File in the root of the repo
#[path="../../syscalls.inc.rs"]
mod values;
#[cfg(arch="amd64")] #[path="raw-amd64.rs"]
mod raw;

#[macro_use]
pub mod logging;

pub mod vfs;
pub mod gui;
pub mod memory;
pub mod threads;


pub use values::WaitItem;

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
	#[allow(dead_code)]
	unsafe fn call_4(&self, call: u16, a1: usize, a2: usize, a3: usize, a4: usize) -> u64 {
		::raw::syscall_4( self.call_value(call), a1, a2, a3, a4 )
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

pub trait Object
{
	const CLASS: u16;
	fn class() -> u16;
	fn from_handle(handle: ObjectHandle) -> Self;
	fn into_handle(self) -> ::ObjectHandle;
	fn get_wait(&self) -> ::values::WaitItem;
	fn check_wait(&self, wi: &::values::WaitItem);
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
pub fn log_write(msg: &str) {
	// SAFE: Syscall
	unsafe { syscall!(CORE_LOGWRITE, msg.as_ptr() as usize, msg.len()); }
}

pub use values::TEXTINFO_KERNEL;

#[inline]
pub fn get_text_info(unit: u32, id: u32, buf: &mut [u8]) -> &str {
	// SAFE: Syscall
	let len: usize = unsafe { syscall!(CORE_TEXTINFO, unit as usize, id as usize,  buf.as_ptr() as usize, buf.len()) } as usize;
	::core::str::from_utf8(&buf[..len]).expect("TODO: get_text_info handle error")
}



