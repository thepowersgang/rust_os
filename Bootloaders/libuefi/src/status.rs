

#[repr(C)]
#[derive(Copy,Clone,PartialEq,Eq)]
/// EFI Status type
pub struct Status(u64);
impl Status
{
	pub fn new(val: u64) -> Status {
		Status(val)
	}
	pub fn err_or<T>(self, v: T) -> Result<T,Status> {
		if self.0 == 0 {
			Ok(v)
		}
		else {
			Err(self)
		}
	}
	pub fn err_or_else<F, T>(self, f: F) -> Result<T,Status>
	where
		F: FnOnce()->T
	{
		if self.0 == 0 {
			Ok( f() )
		}
		else {
			Err(self)
		}
	}

	pub fn message(&self) -> &str {
		match *self
		{
		LOAD_ERROR => "The image failed to load.",
		INVALID_PARAMETER => "A parameter was incorrect.",
		UNSUPPORTED => "The operation is not supported.",
		BAD_BUFFER_SIZE => "The buffer was not the proper size for the request.",
		BUFFER_TOO_SMALL => "The buffer is not large enough to hold the requested data.",
		NOT_FOUND => "The item was not found",
		_ => "?",
		}
	}
}
impl ::core::fmt::Debug for Status
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		match *self
		{
		LOAD_ERROR => write!(f, "Status(LOAD_ERROR {})", self.message()),
		INVALID_PARAMETER => write!(f, "Status(INVALID_PARAMETER {})", self.message()),
		UNSUPPORTED => write!(f, "Status(UNSUPPORTED The operation is not supported.)"),
		BAD_BUFFER_SIZE => write!(f, "Status(BAD_BUFFER_SIZE The buffer was not the proper size for the request.)"),
		BUFFER_TOO_SMALL => write!(f, "Status(BUFFER_TOO_SMALL The buffer is not large enough to hold the requested data.)"),
		NOT_FOUND => write!(f, "Status(NOT_FOUND The item was not found)"),
		_ => write!(f, "Status({:#x})", self.0),
		}
	}
}

pub const SUCCESS: Status = Status(0);
pub const LOAD_ERROR       : Status = Status(1 << 63 | 1);
pub const INVALID_PARAMETER: Status = Status(1 << 63 | 2);
pub const UNSUPPORTED      : Status = Status(1 << 63 | 3);
pub const BAD_BUFFER_SIZE  : Status = Status(1 << 63 | 4);
pub const BUFFER_TOO_SMALL : Status = Status(1 << 63 | 5);
pub const NOT_FOUND        : Status = Status(1 << 63 | 14);

