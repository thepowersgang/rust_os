
use boxed::Box;

pub unsafe fn from_boxed_utf8_unchecked(v: Box<[u8]>) -> Box<str>
{
	Box::from_raw( Box::into_raw(v) as *mut str )
}

impl From<Box<str>> for Box<[u8]>
{
	fn from(v: Box<str>) -> Box<[u8]>
	{
		// SAFE: Same represenation, no extra requirements added
		unsafe { Box::from_raw( Box::into_raw(v) as *mut [u8] ) }
	}
}

