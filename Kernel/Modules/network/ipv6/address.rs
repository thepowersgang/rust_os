

#[derive(Copy,Clone,PartialOrd,PartialEq,Ord,Eq,Debug)]
pub struct Address([u16; 8]);
impl Address {
	pub fn zero() -> Self {
		Address([0; 8])
	}
	pub fn broadcast() -> Self {
		Address([!0; 8])
	}
	pub fn mask_net(&self, prefix_bits: u8) -> Address {
		let mut rv = [0; 8];
		let prefix_whole = prefix_bits as usize / 16;
		for i in 0 .. prefix_whole {
			rv[i] = self.0[i];
		}
		rv[prefix_whole] = self.0[prefix_whole] & (0xFFFF << (prefix_bits % 16));
		Address(rv)
	}
	pub fn mask_host(&self, prefix_bits: u8) -> Address {
		let mut rv = [0; 8];
		let prefix_whole = prefix_bits as usize / 16;
		rv[prefix_whole] = self.0[prefix_whole] & !(0xFFFF << (prefix_bits % 16));
		for i in prefix_whole + 1 .. 8 {
			rv[i] = self.0[i];
		}
		Address(rv)
	}
	pub fn words(&self) -> &[u16; 8] {
		&self.0
	}
	pub fn to_bytes(&self) -> [u8; 16] {
		[
			self.0[0] as u8, (self.0[0] >> 8) as u8,
			self.0[1] as u8, (self.0[1] >> 8) as u8,
			self.0[2] as u8, (self.0[2] >> 8) as u8,
			self.0[3] as u8, (self.0[3] >> 8) as u8,
			self.0[4] as u8, (self.0[4] >> 8) as u8,
			self.0[5] as u8, (self.0[5] >> 8) as u8,
			self.0[6] as u8, (self.0[6] >> 8) as u8,
			self.0[7] as u8, (self.0[7] >> 8) as u8,
		]
	}
	pub fn from_bytes(v: [u8; 16]) -> Self {
		Address([
			(v[0] as u16) | (v[0] as u16) << 8,
			(v[1] as u16) | (v[1] as u16) << 8,
			(v[2] as u16) | (v[2] as u16) << 8,
			(v[3] as u16) | (v[3] as u16) << 8,
			(v[4] as u16) | (v[4] as u16) << 8,
			(v[5] as u16) | (v[5] as u16) << 8,
			(v[6] as u16) | (v[6] as u16) << 8,
			(v[7] as u16) | (v[7] as u16) << 8,
		])
	}
	pub fn from_reader(reader: &mut crate::nic::PacketReader) -> Result<Self,()> {
		Ok( Address::from_bytes(reader.read_bytes([0; 16])?) )
	}
}
impl ::core::fmt::Display for Address {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		for i in 0 .. 8 {
			if i > 0 {
				f.write_str(":")?;
			}
			write!(f, "{:x}", self.0[i])?;
		}
		Ok( () )
	}
}