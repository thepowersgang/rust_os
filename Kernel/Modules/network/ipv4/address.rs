
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
		u32::from_be_bytes(self.0)
	}
	pub fn mask(&self, bits: u8) -> Address {
		assert!(bits <= 32);
		if bits == 0 {
			Address([0; 4])
		}
		else {
			let mask = !0 << (32-bits);
			Address( (u32::from_be_bytes(self.0) & mask).to_be_bytes() )
		}
	}
	pub fn mask_host(&self, bits: u8) -> Address {
		assert!(bits <= 32);
		if bits == 32 {
			*self
		}
		else {
			let mask = !0 << (32-bits);
			Address( (u32::from_be_bytes(self.0) & !mask).to_be_bytes() )
		}
	}
	pub fn is_zero(&self) -> bool {
		self.0 == [0,0,0,0]
	}
}