#![feature(no_std,core,core_prelude,core_str_ext,core_slice_ext)]
#![feature(core_intrinsics)]
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
	fn new(rv: u32) -> Result<ObjectHandle,u32> {
		unimplemented!();
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

