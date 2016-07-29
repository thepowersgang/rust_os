

#[repr(C)]
#[derive(Copy,Clone,PartialEq,Eq)]
/// EFI Status type
pub struct Status(i32);
impl Status
{
	pub fn new(val: i32) -> Status {
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
		NOT_FOUND => write!(f, "Status({} The item was not found.)", self.0),
		_ => write!(f, "Status({})", self.0),
		}
	}
}

pub const NOT_FOUND: Status = Status(14);

