//
//
//
//! Cross-binary interfacing
use core::prelude::*;

pub struct OsStr([u8]);

impl_fmt!{
	Debug(self,f) for OsStr {{
		try!(write!(f, "b\""));
		for &b in &self.0
		{
			match b
			{
			b'\\' => try!(write!(f, "\\\\")),
			b'\n' => try!(write!(f, "\\n")),
			b'\r' => try!(write!(f, "\\r")),
			b'"' => try!(write!(f, "\\\"")),
			b'\0' => try!(write!(f, "\\0")),
			// ASCII printable characters
			32...127 => try!(write!(f, "{}", b as char)),
			_ => try!(write!(f, "\\x{:02x}", b)),
			}
		}
		try!(write!(f, "\""));
		Ok( () )
	}}
}

