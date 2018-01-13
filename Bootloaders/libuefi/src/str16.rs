///
///
///

pub struct Str16([u16]);
impl Str16
{
	/// Converts a UCS-2 (no need to be valid UTF-16) slice into a string
	#[inline]
	pub fn from_slice(s: &[u16]) -> &Str16 {
		// SAFE: Same represenaton as &[u16]
		unsafe {
			::core::mem::transmute(s)
		}
	}

	/// UNSAFE: Indexes input until NUL, lifetime inferred
	#[inline]
	pub unsafe fn from_nul_terminated<'a>(p: *const u16) -> &'a Str16 {
		let len = {
			let mut len = 0;
			while *p.offset(len) != 0
			{
				len += 1;
			}
			len as usize
			};

		let s = ::core::slice::from_raw_parts(p, len);
		Self::from_slice(s)
	}

	/// Obtain an iterator of characters over this string
	/// 
	/// NOTE: Unpaired UTF-16 surrogates are returned as \uFFFD
	#[inline]
	pub fn chars(&self) -> Chars {
		Chars(&self.0)
	}
}
impl ::core::fmt::Display for Str16 {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		for c in self.chars() {
			try!(write!(f, "{}", c));
		}
		Ok( () )
	}
}

pub struct Chars<'a>(&'a [u16]);
impl<'a> Chars<'a>
{
	fn next_codeunit(&mut self) -> Option<u16> {
		if self.0.len() == 0 {
			None
		}
		else {
			let w = self.0[0];
			self.0 = &self.0[1..];
			Some(w)
		}
	}
}
impl<'a> Iterator for Chars<'a>
{
	type Item = char;

	fn next(&mut self) -> Option<char>
	{
		if let Some(w) = self.next_codeunit()
		{
			if 0xD800 <= w && w < 0xDC00 {
				let hi = w - 0xD800;
				if self.0.len() == 0 {
					Some( '\u{fffd}' )
				}
				else if self.0[0] < 0xDC00 {
					Some( '\u{fffd}' )
				}
				else if self.0[0] >= 0xE000 {
					Some( '\u{fffd}' )
				}
				else {
					let w2 = self.next_codeunit().unwrap();
					assert!(0xDC00 <= w2 && w2 < 0xE000);
					let lo = w2 - 0xDC00;
					let cp32 = 0x10000 + (hi as u32) << 10 + lo as u32;
					Some( ::core::char::from_u32(cp32).unwrap() )
				}
			}
			else {
				Some( ::core::char::from_u32(w as u32).unwrap() )
			}
		}
		else
		{
			None
		}
	}
}

/// Pointer to a UCS-2 NUL-terminated string
pub type CStr16Ptr = *const u16;

/// Safe unsized UCS-2 NUL-terminated string type
pub struct CStr16([u16]);
impl CStr16 {
	pub fn as_ptr(&self) -> CStr16Ptr {
		self.0.as_ptr()
	}
	pub fn from_slice(s: &[u16]) -> &CStr16 {
		let l = s.iter().position(|&x| x == 0).expect("No NUL in slice passed to CStr16::from_slice");
		let ss = &s[..l+1];
		// SAFE: Same internal representation, string is NUL terminated
		unsafe { &*(ss as *const [u16] as *const CStr16) }
	}
}

impl<'a> From<&'a [u16]> for &'a CStr16
{
	fn from(v: &'a [u16]) -> Self {
		CStr16::from_slice(v)
	}
}

impl ::core::fmt::Display for CStr16
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		Str16::from_slice(&self.0[.. self.0.len() - 1]).fmt(f)
	}
}

pub struct CString16<'h>(::boot_services::PoolVec<'h, u16>);
impl<'h> ::borrow::ToOwned<'h> for CStr16
{
	type Owned = CString16<'h>;
	fn to_owned(&self, _bs: &'h ::boot_services::BootServices) -> CString16<'h> {
		panic!("CStr16::to_owned");
	}
}
impl<'h> ::borrow::Borrow<CStr16> for CString16<'h>
{
	fn borrow(&self) -> &CStr16 {
		// SAFE: Same internal representation, string is NUL terminated
		unsafe { &*(&*self.0 as *const [u16] as *const CStr16) }
	}
}

