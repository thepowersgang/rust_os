
use core::prelude::*;

const LOG_BUF_SIZE: usize = 256;

struct FixedBuf
{
	len: usize,
	data: [u8; LOG_BUF_SIZE],
}
impl FixedBuf {
	const fn new() -> Self {
		FixedBuf { len: 0, data: [0; LOG_BUF_SIZE] }
	}
	fn clear(&mut self) {
		self.len = 0;
	}
	fn push_back(&mut self, data: &[u8]) {
		let len = self.data[self.len..].clone_from_slice( data );
		self.len += len;
		if len > LOG_BUF_SIZE {
			self.len = 0;
			assert!(self.len <= 128);
		}
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
			Err(_) => {}
			}
			T_LOG_BUFFER.clear();
		}
	}
}

#[macro_export]
macro_rules! kernel_log {
	($($t:tt)+) => { {
		use std::fmt::Write;
		let _ = write!(&mut $crate::logging::ThreadLogWriter, $($t)*);
	} };
}

