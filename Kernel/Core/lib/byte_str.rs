// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/byte_str.rs
//! Byte strings (used for the VFS, and other places where UTF-8 can't be enforced)
use prelude::*;
use core::{cmp,ops,fmt};

#[derive(PartialOrd,Ord,PartialEq,Eq)]
pub struct ByteStr([u8]);
#[derive(PartialOrd,Ord,PartialEq,Eq)]
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

 /*
impl ops::Deref for ByteStr {
	type Target = [u8];
	fn deref(&self) -> &[u8] { &self.0 }
}
// */
impl AsRef<[u8]> for ByteStr {
	fn as_ref(&self) -> &[u8] {
		//unimplemented!(); /*
		&self.0
		// */
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
impl ::lib::borrow::ToOwned for ByteStr {
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
impl<'a> From<&'a ByteStr> for ByteString {
	fn from(v: &'a ByteStr) -> ByteString {
		//unimplemented!(); /*
		ByteString(Vec::from(v.as_bytes()))
		// */
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
		//unimplemented!(); /*
		&self.0
		// */
	}
}
impl ::lib::borrow::Borrow<ByteStr> for ByteString {
	fn borrow(&self) -> &ByteStr {
		//unimplemented!(); /*
		&**self
		// */
	}
}


