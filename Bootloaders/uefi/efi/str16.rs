
pub struct Str16([u16]);
impl Str16
{
	pub fn from_slice(s: &[u16]) -> &Str16 {
		// TODO: Check for valid UTF-16
		// SAFE: Same represenaton as &[u16]
		unsafe {
			::core::mem::transmute(s)
		}
	}

	// UNSAFE: Indexes input until NUL, lifetime inferred
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

	fn next(&mut self) -> Option<char> {
		if let Some(w) = self.next_codeunit()
		{
			if 0xD800 <= w && w < 0xDC00 {
				let hi = w - 0xD800;
				let w2 = self.next_codeunit().unwrap();
				assert!(0xDC00 <= w2 && w2 < 0xE000);
				let lo = w2 - 0xDC00;
				let cp32 = 0x10000 + (hi as u32) << 10 + lo as u32;
				Some( ::core::char::from_u32(cp32).unwrap() )
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

