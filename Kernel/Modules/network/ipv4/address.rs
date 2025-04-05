
#[derive(Copy,Clone,Default,PartialEq,PartialOrd,Eq,Ord)]
pub struct Address(pub [u8; 4]);
impl ::core::fmt::Display for Address
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
	}
}
impl ::core::fmt::Debug for Address {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		::core::fmt::Display::fmt(self, f)
	}
}
impl Address
{
	pub fn zero() -> Self {
		Address([0,0,0,0])
	}
	pub fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
		Address([a,b,c,d])
	}
	/// Big endian u32 (so 127.0.0.1 => 0x7F000001)
	pub fn as_u32(&self) -> u32 {
		(self.0[0] as u32) << 24
		| (self.0[1] as u32) << 16
		| (self.0[2] as u32) << 8
		| (self.0[3] as u32) << 0
	}
	pub fn mask(&self, bits: u8) -> Address {
		let mask = (1 << (bits % 8)) - 1;
		if bits < 8 {
			Address([ self.0[0] & mask, 0, 0, 0 ])
		}
		else if bits < 16 {
			Address([ self.0[0], self.0[1] & mask, 0, 0 ])
		}
		else if bits < 24 {
			Address([ self.0[0], self.0[1], self.0[2] & mask, 0 ])
		}
		else if bits < 32 {
			Address([ self.0[0], self.0[1], self.0[2], self.0[3] & mask ])
		}
		else if bits == 32 {
			Address(self.0)
		}
		else {
			unreachable!()
		}
	}
	pub fn is_zero(&self) -> bool {
		self.0 == [0,0,0,0]
	}
}