
use super::{Status,Event};


pub struct SimpleTextOutputInterface
{
	reset: extern "win64" fn(this: *mut SimpleTextOutputInterface, extended_verification: bool) -> Status,
	output_string: extern "win64" fn(this: *const SimpleTextOutputInterface, string: super::CStr16Ptr) -> Status,
	test_string: extern "win64" fn(this: *mut SimpleTextOutputInterface, string: super::CStr16Ptr) -> Status,
}
impl SimpleTextOutputInterface
{
	pub fn reset(&mut self) -> Status {
		(self.reset)(self, false)
	}
	pub fn output_string(&self, s16: super::CStr16Ptr) -> Status {
		(self.output_string)(self, s16)
	}
}


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
}

