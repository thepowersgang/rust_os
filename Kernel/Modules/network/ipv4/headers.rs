use super::Address;
use crate::nic::PacketReader;

#[allow(dead_code)]
pub struct Ipv4Header
{
	pub ver_and_len: u8,
	pub diff_services: u8,
	pub total_length: u16,
	pub identification: u16,
	pub flags: u8,
	pub frag_ofs_high: u8,
	pub ttl: u8,
	pub protocol: u8,
	pub hdr_checksum: u16,
	pub source: Address,
	pub destination: Address,
}
impl Ipv4Header
{
	pub fn encode(&self) -> [u8; 20]
	{
		[
			self.ver_and_len,
			self.diff_services,
			(self.total_length >> 8) as u8, self.total_length as u8,
			(self.identification >> 8) as u8, self.identification as u8,
			self.flags,
			self.frag_ofs_high,
			self.ttl,
			self.protocol,
			(self.hdr_checksum >> 8) as u8, self.hdr_checksum as u8,
			self.source.0[0], self.source.0[1], self.source.0[2], self.source.0[3],
			self.destination.0[0], self.destination.0[1], self.destination.0[2], self.destination.0[3],
			]
	}
	pub fn set_checksum(&mut self)
	{
		self.hdr_checksum = 0;
		self.hdr_checksum = super::calculate_checksum(self.encode().chunks(2).map(|v| (v[0] as u16) << 8 | v[1] as u16));
	}
	pub fn read(reader: &mut PacketReader) -> Result<Self, ()>
	{
		Ok(Ipv4Header {
			ver_and_len: reader.read_u8()?,
			diff_services: reader.read_u8()?,
			total_length: reader.read_u16n()?,
			identification: reader.read_u16n()?,
			flags: reader.read_u8()?,
			frag_ofs_high: reader.read_u8()?,	// low bits in the `flags` field
			ttl: reader.read_u8()?,
			protocol: reader.read_u8()?,
			hdr_checksum: reader.read_u16n()?,
			source: Address(reader.read_bytes([0; 4])?),
			destination: Address(reader.read_bytes([0; 4])?),
			})
	}

	pub fn get_header_length(&self) -> usize {
		(self.ver_and_len & 0xF) as usize * 4
	}
	pub fn get_has_more_fragments(&self) -> bool {
		self.flags & 1 << 5 != 0
	}
	//fn set_has_more_fragments(&mut self) {
	//	self.flags |= 1 << 5;
	//}

	pub fn get_fragment_ofs(&self) -> usize {
		((self.frag_ofs_high as usize) << 5) | (self.flags & 0x1F) as usize
	}
}