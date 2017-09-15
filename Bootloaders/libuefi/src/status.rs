

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

	#[inline]
	/// Panic if the status isn't SUCCESS
	pub fn unwrap(self) {
		self.unwrap_or( () )
	}
	#[inline]
	/// Return the passed value, or panic if not SUCCESS
	pub fn unwrap_or<T>(self, v: T) -> T {
		self.expect_or("Called unwrap on a non-SUCCESS result", v)
	}
	#[inline]
	pub fn expect(self, msg: &str) {
		self.expect_or( msg, () )
	}
	#[inline]
	pub fn expect_or<T,M: ::core::fmt::Display>(self, msg: M, v: T)->T {
		if self == SUCCESS {
			v
		}
		else {
			panic!("{}: {}", msg, self.message());
		}
	}

	/// Return the official description message for this status value
	pub fn message(&self) -> &str {
		value_to_description(*self).unwrap_or("?")
	}
}

/// Allow `Status` to be used with the `?` operator
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
		// Quick macro to reduce duplication of getting names for each status value
		match value_to_ident(*self)
		{
		Some(v) => f.write_str(v)?,
		None => write!(f, "Status({:#x})", self.0)?,
		}
		f.write_str(" ")?;
		f.write_str(self.message())?;
		f.write_str(")")?;
		Ok( () )
	}
}

macro_rules! status_values {
	($($v:expr => $n:ident $d:expr,)* @ERRORS $($v2:expr => $n2:ident $d2:expr,)*) => {
		$(pub const $n : Status = Status(0 << 63 | $v);)*
		$(pub const $n2 : Status = Status(1 << 63 | $v2);)*

		fn value_to_ident(v: Status)->Option<&'static str> {
			match v
			{
			$($n => Some(stringify!($n)),)*
			$($n2 => Some(stringify!($n2)),)*
			_ => None,
			}
		}
		fn value_to_description(v: Status) -> Option<&'static str> {
			match v
			{
			$($n => Some(stringify!($d1)),)*
			$($n2 => Some(stringify!($d2)),)*
			_ => None,
			}
		}
	}
}
pub const SUCCESS: Status = Status(0);
status_values! {
	1 => WARN_UNKNOWN_GLYPH "The string contained one or more characters that the device could not render and were skipped.",
	2 => WARN_DELETE_FAILURE "The handle was closed, but the file was not deleted.",
	3 => WARN_WRITE_FAILURE "The handle was closed, but the data to the file was not flushed properly.",
	4 => WARN_BUFFER_TOO_SMALL "The resulting buffer was too small, and the data was truncated to the buffer size.",
	5 => WARN_STALE_DATA "The data has not been updated within the timeframe set by local policy for this type of data.",
	6 => WARN_FILE_SYSTEM "The resulting buffer contains UEFI-compliant file system.",
	@ERRORS
	1 => LOAD_ERROR "The image failed to load.",
	2 => INVALID_PARAMETER "A parameter was incorrect.",
	3 => UNSUPPORTED "The operation is not supported.",
	4 => BAD_BUFFER_SIZE "The buffer was not the proper size for the request.",
	5 => BUFFER_TOO_SMALL "The buffer is not large enough to hold the requested data.",
	6 => NOT_READY "There is no data pending upon return.",
	7 => DEVICE_ERROR "The physical device reported an error while attempting the operation.",
	8 => WRITE_PROTECTED "The device cannot be written to.",
	9 => OUT_OF_RESOURCES "A resource has run out.",
	10 => VOLUME_CORRUPTED "An inconstancy was detected on the file system causing the operating to fail.",
	11 => VOLUME_FULL "There is no more space on the file system.",
	12 => NO_MEDIA "The device does not contain any medium to perform the operation.",
	13 => MEDIA_CHANGED "The medium in the device has changed since the last access.",
	14 => NOT_FOUND "The item was not found.",
	15 => ACCESS_DENIED "Access was denied.",
	16 => NO_RESPONSE "The server was not found or did not respond to the request.",
	17 => NO_MAPPING "A mapping to a device does not exist.",
	18 => TIMEOUT "The timeout time expired.",
	19 => NOT_STARTED "The protocol has not been started.",
}

