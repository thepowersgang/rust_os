// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/byte_str.rs
//! Byte strings (used for the VFS, and other places where UTF-8 can't be enforced)
#[allow(unused_imports)]
use crate::prelude::*;
use core::{cmp,ops};

#[derive(PartialOrd,Ord,PartialEq,Eq)]
pub struct ByteStr([u8]);
#[derive(PartialOrd,Ord,PartialEq,Eq,Default,Clone)]
pub struct ByteString(Vec<u8>);

impl ByteStr
{
	pub fn new<T: ?Sized + AsRef<[u8]>>(v: &T) -> &ByteStr {
		// SAFE: as_ref can only return &[u8], and &ByteStr is &[u8]
		unsafe { ::core::mem::transmute(v.as_ref()) }
	}
	
	pub fn len(&self) -> usize { self.0.len() }
	pub fn as_bytes(&self) -> &[u8] { &self.0 }
}
impl AsRef<ByteStr> for str {
	fn as_ref(&self) -> &ByteStr {
		ByteStr::new(self)
	}
}
impl_fmt! {
	Debug(self,f) for ByteStr {{
		write!(f, "b\"")?;
		for &b in &self.0
		{
			match b
			{
			b'\\' => write!(f, "\\\\")?,
			b'\n' => write!(f, "\\n")?,
			b'\r' => write!(f, "\\r")?,
			b'"'  => write!(f, "\\\"")?,
			b'\0' => write!(f, "\\0")?,
			// ASCII printable characters
			32..=127 => write!(f, "{}", b as char)?,
			_ => write!(f, "\\x{:02x}", b)?,
			}
		}
		write!(f, "\"")?;
		Ok( () )
	}}
	Debug(self,f) for ByteString {
		write!(f, "{:?}", &**self)
	}
}

 /*
impl ops::Deref for ByteStr {
	type Target = [u8];
	fn deref(&self) -> &[u8] { &self.0 }
}
// */
impl AsRef<[u8]> for ByteStr {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}
impl cmp::PartialOrd<str> for ByteStr {
	fn partial_cmp(&self, v: &str) -> Option<cmp::Ordering> {
		cmp::PartialOrd::partial_cmp(&self.0, v.as_bytes())
	}
}
impl cmp::PartialEq<str> for ByteStr {
	fn eq(&self, v: &str) -> bool {
		cmp::PartialEq::eq(&self.0, v.as_bytes())
	}
}
impl cmp::PartialEq<[u8]> for ByteStr {
	fn eq(&self, v: &[u8]) -> bool {
		cmp::PartialEq::eq(&self.0, v)
	}
}
impl crate::lib::borrow::ToOwned for ByteStr {
	type Owned = ByteString;
	fn to_owned(&self) -> ByteString {
		ByteString::from(self)
	}
}

impl ByteString
{
	pub fn new() -> ByteString {
		ByteString(Vec::new())
	}
}
impl ::core::iter::FromIterator<u8> for ByteString {
	fn from_iter<T>(iterator: T) -> ByteString
	where
		T: IntoIterator<Item=u8>
	{
		From::<Vec<u8>>::from(iterator.into_iter().collect())
	}
}

impl_from! {
	<('a)> From<&'a [u8]>(v) for ByteString {
		ByteString(Vec::from(v))
	}
	<('a)> From<&'a ByteStr>(v) for ByteString {
		ByteString(Vec::from(v.as_bytes()))
	}
	From< Vec<u8> >(v) for ByteString {
		ByteString(v)
	}
}

impl ops::Deref for ByteString {
	type Target = ByteStr;
	fn deref(&self) -> &ByteStr {
		ByteStr::new(&self.0)
	}
}
impl AsRef<[u8]> for ByteString {
	fn as_ref(&self) -> &[u8] {
		&self.0
	}
}
impl crate::lib::borrow::Borrow<ByteStr> for ByteString {
	fn borrow(&self) -> &ByteStr {
		&**self
	}
}


