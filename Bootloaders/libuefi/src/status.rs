

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
}
impl ::core::fmt::Debug for Status
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		match *self
		{
		NOT_FOUND => write!(f, "Status(NOT_FOUND The item was not found)"),
		LOAD_ERROR => write!(f, "Status(LOAD_ERROR A parameter was incorrect)"),
		INVALID_PARAMETER => write!(f, "Status(INVALID_PARAMETER The operation is not supported)"),
		_ => write!(f, "Status({:#x})", self.0),
		}
	}
}

pub const SUCCESS: Status = Status(0);
pub const LOAD_ERROR: Status = Status(1 << 63 | 1);
pub const INVALID_PARAMETER: Status = Status(1 << 63 | 2);
pub const NOT_FOUND: Status = Status(1 << 63 | 14);

