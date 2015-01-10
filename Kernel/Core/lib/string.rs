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
	pub fn with_capacity(cap: uint) -> String {
		String(Vec::with_capacity(cap))
	}
	pub fn from_str(string: &str) -> String {
		let mut v = Vec::new();
		v.push_all(string.as_bytes());
		String(v)
	}
	
	pub fn as_slice(&self) -> &str {
		let &String(ref v) = self;
		unsafe { ::core::mem::transmute( v.as_slice() ) }
	}
}

impl ::core::fmt::String for String
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> Result<(),::core::fmt::Error>
	{
		self.as_slice().fmt(f)
	}
}

// vim: ft=rust
