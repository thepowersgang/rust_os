

#[repr(C)]
#[derive(Copy,Clone,PartialEq,Eq)]
/// EFI Status type
pub struct Status(u64);
impl Status
{
	#[inline]
	pub fn new(val: u64) -> Status {
		Status(val)
	}
	#[inline]
	pub fn err_or<T>(self, v: T) -> Result<T,Status> {
		if self.0 == 0 {
			Ok(v)
		}
		else {
			Err(self)
		}
	}
	#[inline]
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
		SUCCESS => "Success",
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

impl ::core::ops::Try for Status
{
	type Ok = ();
	type Error = Status;

	fn into_result(self) -> Result<(), Status> {
		if self == SUCCESS {
			Ok( () )
		}
		else {
			Err(self)
		}
	}
	fn from_error(v: Status) -> Status {
		v
	}
	fn from_ok(_: ()) -> Status {
		SUCCESS
	}
}

impl ::core::fmt::Debug for Status
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		f.write_str("Status(")?;
		match *self
		{
		SUCCESS    => f.write_str("SUCCESS")?,
		LOAD_ERROR => f.write_str("LOAD_ERROR")?,
		INVALID_PARAMETER => f.write_str("INVALID_PARAMETER")?,
		UNSUPPORTED       => f.write_str("UNSUPPORTED")?,
		BAD_BUFFER_SIZE   => f.write_str("BAD_BUFFER_SIZE")?,
		BUFFER_TOO_SMALL  => f.write_str("BUFFER_TOO_SMALL")?,
		NOT_FOUND         => f.write_str("NOT_FOUND")?,
		_ => write!(f, "Status({:#x})", self.0)?,
		}
		f.write_str(" ")?;
		f.write_str(self.message())?;
		f.write_str(")")?;
		Ok( () )
	}
}

pub const SUCCESS: Status = Status(0);
pub const LOAD_ERROR       : Status = Status(1 << 63 | 1);
pub const INVALID_PARAMETER: Status = Status(1 << 63 | 2);
pub const UNSUPPORTED      : Status = Status(1 << 63 | 3);
pub const BAD_BUFFER_SIZE  : Status = Status(1 << 63 | 4);
pub const BUFFER_TOO_SMALL : Status = Status(1 << 63 | 5);
pub const NOT_FOUND        : Status = Status(1 << 63 | 14);

