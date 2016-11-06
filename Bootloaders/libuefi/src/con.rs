
use super::{Status,Event};

/// ::core::fmt::Write object for logging via the UEFI SimpleTextOutputInterface
pub struct EfiLogger<'a>(&'a SimpleTextOutputInterface);
impl<'a> EfiLogger<'a> {
	pub fn new(i: &SimpleTextOutputInterface) -> EfiLogger {
		EfiLogger(i)
	}
	fn write_char(&mut self, c: char) {
		let mut b = [0, 0, 0];
		c.encode_utf16(&mut b);
		self.0.output_string( b.as_ptr() );
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
		self.0.output_string( [b'\r' as u16, b'\n' as u16, 0].as_ptr() );
	}
}

/// UEFI Simple Text Output (e.g. serial or screen)
pub struct SimpleTextOutputInterface
{
	reset: extern "win64" fn(this: *mut SimpleTextOutputInterface, extended_verification: bool) -> Status,
	output_string: extern "win64" fn(this: *const SimpleTextOutputInterface, string: super::CStr16Ptr) -> Status,
	test_string: extern "win64" fn(this: *const SimpleTextOutputInterface, string: super::CStr16Ptr) -> Status,
}
impl SimpleTextOutputInterface
{
	/// Reset the console
	pub fn reset(&mut self) -> Status {
		(self.reset)(self, false)
	}
	/// Print the passed string to the console
	pub fn output_string(&self, s16: super::CStr16Ptr) -> Status {
		(self.output_string)(self, s16)
	}
	/// ?? TODO
	pub fn test_string(&self, s16: super::CStr16Ptr) -> Status {
		(self.test_string)(self, s16)
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
	wait_for_key: Event,
}

impl SimpleInputInterface
{
	pub fn reset(&mut self) -> Status {
		(self.reset)(self, false)
	}

	pub fn read_key_stroke(&mut self) -> Result<InputKey, Status> {
		let mut ik = Default::default();
		let s = (self.read_key_stroke)(self, &mut ik);
		s.err_or(ik)
	}
}

