use core::{ops,fmt};

/// String backed to a statically-allocated buffer
pub struct FixedString<Buf: AsMut<[u8]>+AsRef<[u8]>>
{
	data: Buf,
	len: usize,
}

impl<B: AsMut<[u8]>+AsRef<[u8]>> FixedString<B>
{
	/// Create a new fixed-capacity string using the provided buffer
	pub fn new(backing: B) -> FixedString<B> {
		assert!(backing.as_ref().len() > 0);
		FixedString {
			data: backing,
			len: 0,
		}
	}
	pub fn push_char(&mut self, c: char) {
		if c.len_utf8() > self.data.as_ref().len() - self.len {
			todo!("Freeze string once allocation exceeded");
		}
		let l = c.encode_utf8(&mut self.data.as_mut()[self.len..]).len();
		self.len += l
	}
	/// Append a slice
	pub fn push_str(&mut self, s: &str) {
		self.extend( s.chars() );
	}
	
	pub fn clear(&mut self) {
		self.len = 0;
	}
}
impl<B: AsMut<[u8]>+AsRef<[u8]>> ::core::iter::Extend<char> for FixedString<B>
{
	fn extend<T>(&mut self, iterable: T)
	where
		T: ::core::iter::IntoIterator<Item=char>
	{
		for c in iterable {
			self.push_char(c);
		}
	}
}
impl<B: AsMut<[u8]>+AsRef<[u8]>> ops::Deref for FixedString<B>
{
	type Target = str;
	fn deref(&self) -> &str {
		let bytes = &self.data.as_ref()[..self.len];
		// SAFE: String bytes are valid UTF-8
		unsafe { ::core::str::from_utf8_unchecked(bytes) }
	}
}
impl<B: AsMut<[u8]>+AsRef<[u8]>> fmt::Display for FixedString<B> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Display::fmt(&**self, f)
	}
}
impl<B: AsMut<[u8]>+AsRef<[u8]>> fmt::Write for FixedString<B> {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		self.push_str(s);
		Ok( () )
	}
}
