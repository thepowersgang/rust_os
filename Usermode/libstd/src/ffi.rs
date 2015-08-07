//
//
//
//! Cross-binary interfacing

pub struct OsStr([u8]);

impl OsStr
{
	pub fn new<'a, S: AsRef<[u8]>+'a>(s: S) -> &'a OsStr {
		// SAFE: OsStr is [u8]
		unsafe { ::core::mem::transmute(s.as_ref()) }
	}
}
impl AsRef<[u8]> for OsStr {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}

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

