//
//
//
//! Cross-binary interfacing
use alloc::vec::Vec;

pub struct OsStr([u8]);
#[derive(Clone)]
pub struct OsString(Vec<u8>);

impl OsStr
{
	pub fn new<'a, S: AsRef<[u8]>+'a>(s: S) -> &'a OsStr {
		// SAFE: OsStr is [u8]
		unsafe { ::core::mem::transmute(s.as_ref()) }
	}

	pub fn as_bytes(&self) -> &[u8] {
		self.as_ref()
	}

	pub fn to_str(&self) -> Option<&str> {
		::str::from_utf8(self.as_bytes()).ok()
	}
	pub fn to_str_lossy(&self) -> ::borrow::Cow<str> {
		::string::String::from_utf8_lossy(&self.0)
	}
}
impl AsRef<[u8]> for OsStr {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}
impl AsRef<OsStr> for OsStr {
	fn as_ref(&self) -> &OsStr {
		self
	}
}
impl AsRef<OsStr> for [u8] {
	fn as_ref(&self) -> &OsStr {
		OsStr::new(self)
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
			32..=127 => try!(write!(f, "{}", b as char)),
			_ => try!(write!(f, "\\x{:02x}", b)),
			}
		}
		try!(write!(f, "\""));
		Ok( () )
	}}
	Debug(self,f) for OsString {
		self.as_os_str().fmt(f)
	}
}
impl ::core::cmp::PartialEq for OsStr {
	fn eq(&self, other: &OsStr) -> bool {
		&self.0 == &other.0
	}
}
impl ::core::cmp::PartialEq<str> for OsStr {
	fn eq(&self, other: &str) -> bool {
		&self.0 == other.as_bytes()
	}
}

impl OsString {
	pub fn new() -> OsString {
		OsString(Vec::new())
	}
	pub fn as_os_str(&self) -> &OsStr {
		&self
	}
}
impl ::core::ops::Deref for OsString {
	type Target = OsStr;
	fn deref(&self) -> &OsStr {
		OsStr::new(&self.0)
	}
}
impl<'a> From<Vec<u8>> for OsString {
	fn from(v: Vec<u8>) -> OsString {
		OsString(v)
	}
}
impl<'a, T: 'a + ?Sized + AsRef<OsStr>> From<&'a T> for OsString {
	fn from(v: &T) -> OsString {
		OsString(From::from(&v.as_ref().0))
	}
}
impl ::core::cmp::PartialEq<str> for OsString {
	fn eq(&self, other: &str) -> bool {
		&*self.0 == other.as_bytes()
	}
}

