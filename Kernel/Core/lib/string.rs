//
//
//
//
use _common::*;

pub struct String(Vec<u8>);

impl String
{
	pub fn new() -> String {
		String(Vec::new())
	}
	pub fn with_capacity(cap: usize) -> String {
		String(Vec::with_capacity(cap))
	}
	pub fn from_str(string: &str) -> String {
		let mut v = Vec::new();
		v.push_all(string.as_bytes());
		String(v)
	}
	pub fn from_args(args: ::core::fmt::Arguments) -> String {
		use core::fmt::Write;
		let mut ret = String::new();
		let _ = write!(&mut ret, "{}", args);
		ret
	}
	
	pub fn push(&mut self, s: &str)
	{
		self.0.push_all(s.as_bytes());
	}
	
	pub fn as_slice(&self) -> &str {
		let &String(ref v) = self;
		unsafe { ::core::mem::transmute( v.as_slice() ) }
	}
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

#[macro_export]
macro_rules! format {
	($($arg:tt)*) => ($crate::lib::string::String::from_args(format_args!($($arg)*)))
}

// vim: ft=rust
