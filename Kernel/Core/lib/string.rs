// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/string.rs
//! Dynamically-allocated string type
//!
//! Acts every similarly to the rust std's String type.
use crate::prelude::*;
use core::{ops,cmp,fmt};

/// String type
#[derive(Clone,PartialOrd,Ord,PartialEq,Eq)]
pub struct String(Vec<u8>);

impl String
{
	/// Create a new empty string (with no allocation)
	pub fn new() -> String {
		String(Vec::new())
	}
	/// Create a pre-allocated string capable of holding `cap` bytes
	pub fn with_capacity(cap: usize) -> String {
		String(Vec::with_capacity(cap))
	}
	/// Create a string from a string slice
	pub fn from_str(string: &str) -> String {
		let mut v = Vec::new();
		v.extend_from_slice(string.as_bytes());
		String(v)
	}
	/// Create a string from a `fmt::Arguments` instance (used by `format!`)
	pub fn from_args(args: fmt::Arguments) -> String {
		use core::fmt::Write;
		let mut ret = String::new();
		let _ = write!(&mut ret, "{}", args);
		ret
	}
	
	/// Append `s` to the string
	pub fn push_str(&mut self, s: &str)
	{
		self.0.extend_from_slice(s.as_bytes());
	}
	
	/// Return the string as a &str
	fn as_slice(&self) -> &str {
		let bytes: &[u8] = self.0.as_ref();
		// SAFE: Bytes are valid UTF-8
		unsafe { ::core::str::from_utf8_unchecked( bytes ) }
	}
}

/// Construct an empty string
impl Default for String {
	fn default() -> String { String::new() }
}

impl fmt::Write for String
{
	fn write_str(&mut self, s: &str) -> ::core::fmt::Result
	{
		self.push_str(s);
		Ok( () )
	}
}

impl ops::Deref for String
{
	type Target = str;
	fn deref(&self) -> &str {
		self.as_slice()
	}
}

impl fmt::Display for String
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		<str as fmt::Display>::fmt(&self, f)
	}
}
impl fmt::Debug for String
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		<str as fmt::Debug>::fmt(&self, f)
	}
}

impl<'a> From<&'a str> for String
{
	fn from(v: &str) -> String {
		String::from_str(v)
	}
}
//impl<T: ::lib::error::Error> From<T> for String {
//	fn from(v: T) -> String {
//		format!("{}", v)
//	}
//}

impl PartialEq<str> for String {
	fn eq(&self, v: &str) -> bool {
		<str as PartialEq>::eq(&self, v)
	}
}
impl PartialOrd<str> for String {
	fn partial_cmp(&self, v: &str) -> Option<cmp::Ordering> {
		<str as PartialOrd>::partial_cmp(&self, v)
	}
}
impl<'a> PartialEq<&'a str> for String {
	fn eq(&self, v: & &'a str) -> bool {
		<str as PartialEq>::eq(&self, *v)
	}
}
impl<'a> PartialOrd<&'a str> for String {
	fn partial_cmp(&self, v: & &'a str) -> Option<cmp::Ordering> {
		<str as PartialOrd>::partial_cmp(&self, *v)
	}
}



/// Construct a `String` using a format string and arguments
#[macro_export]
macro_rules! format {
	($($arg:tt)*) => ($crate::lib::string::String::from_args(format_args!($($arg)*)))
}

// vim: ft=rust
