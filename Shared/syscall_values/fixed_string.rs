
/// Fixed-capacity string buffer (6 bytes)
#[derive(Copy,Clone)]
pub struct FixedStr6([u8; 6]);
impl ::core::fmt::Debug for FixedStr6 {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		::core::fmt::Debug::fmt(&**self, f)
	}
}
impl ::core::ops::Deref for FixedStr6 {
	type Target = str;
	#[inline]
	fn deref(&self) -> &str { ::core::str::from_utf8(&self.0).expect("Invalid UTF-8 from kernel").split('\0').next().unwrap() }
}
impl<'a> ::core::convert::From<&'a str> for FixedStr6 {
	fn from(v: &str) -> FixedStr6 { From::from(v.as_bytes()) }
}
impl<'a> ::core::convert::From<&'a [u8]> for FixedStr6 {
	fn from(v: &[u8]) -> FixedStr6 {
		let mut rv = [0; 6];
		assert!(v.len() <= 6);
		rv[..v.len()].clone_from_slice(v);
		FixedStr6(rv)
	}
}
impl ::core::convert::From<[u8; 6]> for FixedStr6 {
	fn from(v: [u8; 6]) -> FixedStr6 {
		FixedStr6(v)
	}
}
/// Fixed-capacity string buffer (8 bytes)
#[derive(Copy,Clone)]
pub struct FixedStr8([u8; 8]);
impl ::core::fmt::Debug for FixedStr8 {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		::core::fmt::Debug::fmt(&**self, f)
	}
}
impl ::core::ops::Deref for FixedStr8 {
	type Target = str;
	#[inline]
	fn deref(&self) -> &str { ::core::str::from_utf8(&self.0).expect("Invalid UTF-8 from kernel").split('\0').next().unwrap() }
}
impl<'a> ::core::convert::From<&'a str> for FixedStr8 {
	fn from(v: &str) -> FixedStr8 { From::from(v.as_bytes()) }
}
impl<'a> ::core::convert::From<&'a [u8]> for FixedStr8 {
	fn from(v: &[u8]) -> FixedStr8 {
		let mut rv = [0; 8];
		assert!(v.len() <= 8);
		rv[..v.len()].clone_from_slice(v);
		FixedStr8(rv)
	}
}
impl ::core::convert::From<[u8; 8]> for FixedStr8 {
	fn from(v: [u8; 8]) -> FixedStr8 {
		FixedStr8(v)
	}
}
impl ::core::convert::From<u64> for FixedStr8 {
	fn from(v: u64) -> FixedStr8 {
		FixedStr8( v.to_ne_bytes() ) 
	}
}
impl ::core::convert::From<FixedStr8> for u64 {
	fn from(v: FixedStr8) -> u64 {
		u64::from_ne_bytes(v.0)
	}
}