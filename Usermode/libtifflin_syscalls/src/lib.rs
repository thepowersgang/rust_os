#![feature(no_std,core)]
#![feature(asm)]
#![feature(thread_local,const_fn)]
#![no_std]

use core::prelude::*;
#[macro_use]
extern crate core;

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
}

// File in the root of the repo
#[path="../../../syscalls.inc.rs"]
mod values;
#[cfg(arch__amd64)] #[path="raw-amd64.rs"]
mod raw;

#[macro_use]
pub mod logging;

pub mod vfs;

pub struct ObjectHandle(u32);
impl ObjectHandle
{
	fn new(rv: usize) -> Result<ObjectHandle,u32> {
		to_result(rv).map( |v| ObjectHandle(v) )
	}
	unsafe fn call_0(&self, call: u16) -> u64 {
		::raw::syscall_0( (1 << 31 | self.0 | (call as u32) << 20) )
	}
	unsafe fn call_1(&self, call: u16, arg1: usize) -> u64 {
		::raw::syscall_1( (1 << 31 | (call as u32) << 20 | self.0), arg1)
	}
	unsafe fn call_2(&self, call: u16, arg1: usize, arg2: usize) -> u64 {
		::raw::syscall_2( (1 << 31 | (call as u32) << 20 | self.0), arg1, arg2 )
	}
	unsafe fn call_3(&self, call: u16, arg1: usize, arg2: usize, arg3: usize) -> u64 {
		::raw::syscall_3( (1 << 31 | (call as u32) << 20 | self.0), arg1, arg2, arg3 )
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

fn to_result(val: usize) -> Result<u32,u32> {
	if val < usize::max_value()/2 {
		Ok(val as u32)
	}
	else {
		Err( (val - usize::max_value()/2) as u32 )
	}
}

#[inline]
pub fn log_write(msg: &str) {
	unsafe {
		syscall!(CORE_LOGWRITE, msg.as_ptr() as usize, msg.len());
	}
}
#[inline]
pub fn exit(code: u32) -> ! {
	unsafe {
		syscall!(CORE_EXITPROCESS, code as usize);
		::core::intrinsics::unreachable();
	}
}

