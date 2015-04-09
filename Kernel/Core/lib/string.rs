// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/string.rs
//! Dynamically-allocated string type
//!
//! Acts every similarly to the rust std's String type.
use _common::*;

/// String type
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
		v.push_all(string.as_bytes());
		String(v)
	}
	/// Create a string from a `fmt::Arguments` instance (used by `format!`)
	pub fn from_args(args: ::core::fmt::Arguments) -> String {
		use core::fmt::Write;
		let mut ret = String::new();
		let _ = write!(&mut ret, "{}", args);
		ret
	}
	
	/// Append `s` to the string
	pub fn push(&mut self, s: &str)
	{
		self.0.push_all(s.as_bytes());
	}
	
	/// Return the string as a &str
	pub fn as_slice(&self) -> &str {
		let &String(ref v) = self;
		unsafe { ::core::mem::transmute( v.as_slice() ) }
	}
}

impl ::core::default::Default for String
{
	fn default() -> String { String::new() }
}

impl ::core::fmt::Write for String
{
	fn write_str(&mut self, s: &str) -> ::core::fmt::Result
	{
		self.push(s);
		Ok( () )
	}
}

impl ::core::ops::Deref for String
{
	type Target = str;
	fn deref(&self) -> &str
	{
		let &String(ref v) = self;
		unsafe { ::core::mem::transmute( v.as_slice() ) }
	}
}

impl ::core::fmt::Display for String
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		::core::fmt::Display::fmt(self.as_slice(), f)
	}
}

impl<'a> From<&'a str> for String
{
	fn from(v: &str) -> String {
		String::from_str(v)
	}
}

/// Construct a `String` using a format string and arguments
#[macro_export]
macro_rules! format {
	($($arg:tt)*) => ($crate::lib::string::String::from_args(format_args!($($arg)*)))
}

// vim: ft=rust
