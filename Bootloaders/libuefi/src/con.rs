
use super::{Status,status};

/// ::core::fmt::Write object for logging via the UEFI SimpleTextOutputInterface
pub struct EfiLogger<'a>(&'a SimpleTextOutputInterface);
impl<'a> EfiLogger<'a> {
	pub fn new(i: &SimpleTextOutputInterface) -> EfiLogger {
		EfiLogger(i)
	}
	fn write_char(&mut self, c: char) {
		let mut b = [0, 0, 0];
		c.encode_utf16(&mut b);
		// SAFE: NUL terminated valid pointer
		unsafe {
			self.0.output_string( b.as_ptr() );
		}
	}
}
impl<'a> ::core::fmt::Write for EfiLogger<'a> {
	fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
		for c in s.chars() {
			self.write_char(c);
		}
		Ok( () )
	}

	fn write_fmt(&mut self, a: ::core::fmt::Arguments) -> ::core::fmt::Result {
		::core::fmt::write(self, a)
	}
}
impl<'a> Drop for EfiLogger<'a> {
	fn drop(&mut self) {
		// SAFE: NUL terminated valid pointer
		unsafe {
			self.0.output_string( [b'\r' as u16, b'\n' as u16, 0].as_ptr() );
		}
	}
}

/// UEFI Simple Text Output (e.g. serial or screen)
pub struct SimpleTextOutputInterface
{
	reset: efi_fcn!{ fn(this: *mut SimpleTextOutputInterface, extended_verification: bool) -> Status},
	output_string: efi_fcn!{ fn(this: *const SimpleTextOutputInterface, string: super::CStr16Ptr) -> Status },
	test_string: unsafe extern "win64" fn(this: *const SimpleTextOutputInterface, string: super::CStr16Ptr) -> Status,
}
impl SimpleTextOutputInterface
{
	/// Reset the console
	#[inline]
	pub fn reset(&mut self) -> Status {
		// SAFE: Call cannot cause memory unsafety
		unsafe { 
			(self.reset)(self, false)
		}
	}
	/// Print the passed string to the console
	#[inline]
	pub unsafe fn output_string(&self, s16: super::CStr16Ptr) -> Status {
		(self.output_string)(self, s16)
	}
	/// ?? TODO
	#[inline]
	pub unsafe fn test_string(&self, s16: super::CStr16Ptr) -> Status {
		(self.test_string)(self, s16)
	}

	/// Helper - Print the passed rust string to the console (does multiple calls to `output_string`)
	pub fn output_string_utf8(&self, s: &str) -> Status {
		for c in s.chars() {
			let mut s16 = [0, 0, 0];
			c.encode_utf16(&mut s16);
			// SAFE: NUL terminated valid pointer
			unsafe {
				let r = self.output_string( s16.as_ptr() );
				if r != status::SUCCESS {
					return r;
				}
			}
		}
		status::SUCCESS
	}
}

#[derive(Default)]
pub struct InputKey
{
	scan_code: u16,
	unicode_char: u16,
}

#[repr(C)]
pub struct SimpleInputInterface
{
	reset: extern "win64" fn(this: *mut SimpleInputInterface, extended_verification: bool) -> Status,
	read_key_stroke: extern "win64" fn(this: *mut SimpleInputInterface, keyout: &mut InputKey) -> Status,
	wait_for_key: ::boot_services::raw::Event,
}

impl SimpleInputInterface
{
	#[inline]
	pub fn reset(&mut self) -> Status {
		(self.reset)(self, false)
	}

	#[inline]
	pub fn read_key_stroke(&mut self) -> Result<InputKey, Status> {
		let mut ik = Default::default();
		let s = (self.read_key_stroke)(self, &mut ik);
		s.err_or(ik)
	}
}

