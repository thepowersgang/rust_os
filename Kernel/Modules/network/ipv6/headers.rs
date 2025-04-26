use super::Address;

#[allow(dead_code)]
pub struct Ipv6Header
{
	pub ver_tc_fl: u32,
	pub payload_length: u16,
	/// Type of next protocol header, same as IPV4's `protocol` field
	pub next_header: u8,
	/// Same as IPv4's TTL
	pub hop_limit: u8,

	pub source: Address,
	pub destination: Address,
}
impl Ipv6Header
{
	pub fn encode(&self) -> [u8; 40]
	{
		let mut rv = [0; 8 + 16+16];
		let mut i = 0;
		let mut push = |v: &[u8]| {
			rv[i..][..v.len()].copy_from_slice(v);
			i += v.len();
		};
		push(&self.ver_tc_fl.to_be_bytes());
		push(&self.payload_length.to_be_bytes());
		push(&self.next_header.to_be_bytes());
		push(&self.hop_limit.to_be_bytes());
		push(&self.source.to_bytes());
		push(&self.destination.to_bytes());
		assert!(i == rv.len());
		rv
	}
	pub fn read(reader: &mut crate::nic::PacketReader) -> Result<Self, ()>
	{
		Ok(Ipv6Header {
			ver_tc_fl: reader.read_u32n()?,
			payload_length: reader.read_u16n()?,
			next_header: reader.read_u8()?,
			hop_limit: reader.read_u8()?,
			source: Address::from_bytes(reader.read_bytes([0; 16])?),
			destination: Address::from_bytes(reader.read_bytes([0; 16])?),
			})
	}
}
