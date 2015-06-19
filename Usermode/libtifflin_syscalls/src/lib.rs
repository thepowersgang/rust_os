#![feature(no_std,core)]
#![feature(asm)]
#![feature(thread_local,const_fn)]
#![no_std]

use core::prelude::*;
#[macro_use]
extern crate core;

#[repr(u32,C)]
enum Syscalls {
	LogWrite = 0x0_0000,
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

struct FixedBuf
{
	len: usize,
	data: [u8; 128],
}
impl FixedBuf {
	const fn new() -> Self {
		FixedBuf { len: 0, data: [0; 128] }
	}
	fn clear(&mut self) {
		self.len = 0;
	}
	fn push_back(&mut self, data: &[u8]) {
		let len = self.data[self.len..].clone_from_slice( data );
		self.len += len;
		assert!(self.len <= 128);
	}
}
impl ::core::ops::Deref for FixedBuf {
	type Target = [u8];
	fn deref(&self) -> &[u8] {
		&self.data[..self.len]
	}
}

//#[thread_local]
static mut T_LOG_BUFFER: FixedBuf = FixedBuf::new();

// A simple writer that uses the kernel-provided per-thread logging channel
pub struct ThreadLogWriter;
impl ::core::fmt::Write for ThreadLogWriter {
	fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
		// SAFE: Thread-local
		unsafe {
			T_LOG_BUFFER.push_back(s.as_bytes());
		}
		Ok( () )
	}
}
impl ::core::ops::Drop for ThreadLogWriter {
	fn drop(&mut self) {
		// SAFE: Thread-local
		unsafe {
			let b = &*T_LOG_BUFFER;
			match ::core::str::from_utf8(b)
			{
			Ok(v) => ::log_write(v),
			Err(e) => {}
			}
			T_LOG_BUFFER.clear();
		}
	}
}

#[macro_export]
macro_rules! kernel_log {
	($($t:tt)+) => { {
		use std::fmt::Write;
		let _ = write!(&mut $crate::ThreadLogWriter, $($t)*);
	} };
}

#[inline]
pub fn log_write(msg: &str) {
	unsafe {
		syscall!(LogWrite, msg.as_ptr() as usize, msg.len());
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
	macro_rules! syscall_a {
		($id:expr, $( $reg:tt = $val:expr),*) => {{
			let rv;
			asm!("syscall"
				: "={rax}" (rv)
				: "{rax}" ($id) $(, $reg ($val))*
				: "rcx", "r11"
				: "volatile"
				);
			rv
		}};
	}
	pub unsafe fn syscall_0(id: u32) -> u64 {
		syscall_a!(id, )
	}
	pub unsafe fn syscall_1(id: u32, a1: usize) -> u64 {
		syscall_a!(id, "{rdi}"=a1)
	}
	pub unsafe fn syscall_2(id: u32, a1: usize, a2: usize) -> u64 {
		syscall_a!(id, "{rdi}"=a1, "{rsi}"=a2)
	}
}

