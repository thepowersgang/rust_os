#![feature(no_std,core)]
#![feature(asm)]
#![no_std]

use core::prelude::*;
extern crate core;

#[repr(u32,C)]
enum Syscalls {
	LogWrite = 0x0_0000,
	LogCommit,
	ExitProcess,
}

macro_rules! syscall {
	($id:ident) => {
		arch::syscall_0(Syscalls::$id as u32)
		};
	($id:ident, $arg1:expr) => {
		::arch::syscall_1(Syscalls::$id as u32, $arg1)
		};
	($id:ident, $arg1:expr, $arg2:expr) => {
		::arch::syscall_2(Syscalls::$id as u32, $arg1, $arg2)
		};
}


// A simple writer that uses the kernel-provided per-thread logging channel
pub struct ThreadLogWriter;
impl ::core::fmt::Write for ThreadLogWriter {
	fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
		::log_write(s);
		Ok( () )
	}
}
impl ::core::ops::Drop for ThreadLogWriter {
	fn drop(&mut self) {
		::log_commit();
	}
}


#[inline]
pub fn log_write(msg: &str) {
	unsafe {
		syscall!(LogWrite, msg.len(), msg.as_ptr() as usize);
	}
}
#[inline]
pub fn log_commit() {
	unsafe {
		syscall!(LogCommit);
	}
}
#[inline]
pub fn exit(code: u32) -> ! {
	unsafe {
		syscall!(ExitProcess, code as usize);
		::core::intrinsics::unreachable();
	}
}

#[cfg(arch__amd64)]
mod arch
{
	pub unsafe fn syscall_0(id: u32) -> u64 {
		let rv;
		asm!("syscall" : "={rax}" (rv) : "{rax}" (id));
		rv
	}
	pub unsafe fn syscall_1(id: u32, a1: usize) -> u64 {
		let rv;
		asm!("syscall" : "={rax}" (rv) : "{rax}" (id), "{rsi}" (a1));
		rv
	}
	pub unsafe fn syscall_2(id: u32, a1: usize, a2: usize) -> u64 {
		let rv;
		asm!("syscall" : "={rax}" (rv) : "{rax}" (id), "{rsi}" (a1), "{rdi}" (a2));
		rv
	}
}

