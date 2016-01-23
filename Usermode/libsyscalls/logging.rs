
const LOG_BUF_SIZE: usize = 256;

pub struct FixedBuf
{
	len: usize,
	data: [u8; LOG_BUF_SIZE],
}
impl FixedBuf {
	pub const fn new() -> Self {
		FixedBuf { len: 0, data: [0; LOG_BUF_SIZE] }
	}
	fn clear(&mut self) {
		self.len = 0;
	}
	fn push_back(&mut self, data: &[u8]) {
		assert!( data.len() <= self.data.len() - self.len, "Pushed too much to FixedBuf" );
		self.data[self.len..].clone_from_slice( data );
		self.len += data.len();;
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
pub struct ThreadLogWriter<'a>(&'a mut FixedBuf);
impl<'a> ThreadLogWriter<'a> {
	pub fn new(b: &mut FixedBuf) -> ThreadLogWriter {
		ThreadLogWriter(b)
	}
}
impl<'a> ::core::fmt::Write for ThreadLogWriter<'a> {
	fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
		self.0.push_back(s.as_bytes());
		Ok( () )
	}
}
impl<'a> ::core::ops::Drop for ThreadLogWriter<'a> {
	fn drop(&mut self) {
		::log_write( &**self.0 );
		self.0.clear();
	}
}

#[inline(never)]
#[doc(hidden)]
pub fn write<F: ::core::ops::FnOnce(&mut ::logging::ThreadLogWriter)->::core::fmt::Result>(fcn: F) {
	// SAFE: Thread-local
	let _ = fcn(&mut ::logging::ThreadLogWriter(unsafe { &mut T_LOG_BUFFER }));
}

// NOTE: Calls the above function with a closure to prevent the caller's stack frame from balooning with the formatting junk
#[macro_export]
macro_rules! kernel_log {
	($($t:tt)+) => { {
		$crate::logging::write(|s| { use std::fmt::Write; write!(s, $($t)*) });
	} };
}

