
pub trait AsciiExt
{
	type Owned;
	
	fn is_ascii(&self) -> bool;
	fn to_ascii_uppercase(&self) -> Self::Owned;
	fn to_ascii_lowercase(&self) -> Self::Owned;
}

impl AsciiExt for u8
{
	type Owned = u8;
	fn is_ascii(&self) -> bool { *self < 0x7F }
	fn to_ascii_uppercase(&self) -> u8 {
		match *self
		{
		v @ b'a' ... b'z' => v - 0x20,
		v @ _ => v,
		}
	}
	fn to_ascii_lowercase(&self) -> u8 {
		match *self
		{
		v @ b'A' ... b'Z' => v + 0x20,
		v @ _ => v,
		}
	}
}

