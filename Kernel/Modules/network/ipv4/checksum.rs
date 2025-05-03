
// Calculate a checksum of a sequence of NATIVE ENDIAN (not network) 16-bit words
pub fn from_words(words: impl Iterator<Item=u16>) -> u16
{
	let mut sum = 0;
	for v in words
	{
		sum += v as usize;
	}
	while sum > 0xFFFF
	{
		sum = (sum & 0xFFFF) + (sum >> 16);
	}
	!sum as u16
}

pub fn from_bytes(bytes: impl Iterator<Item=u8>) -> u16
{
	struct Words<I>(I);
	impl<I> Iterator for Words<I>
	where I: Iterator<Item=u8>
	{
		type Item = u16;
		
		fn next(&mut self) -> Option<Self::Item> {
			// NOTE: This only really works on fused iterators
			match (self.0.next(),self.0.next()) {
			(Some(a),Some(b)) => Some(u16::from_be_bytes([a,b])),
			(Some(a),None) => Some(u16::from_be_bytes([a,0])),
			(None,_) => None,
			}
		}
	}

	from_words(Words(bytes.fuse()))
}

pub fn from_reader(mut reader: crate::nic::PacketReader) -> u16
{
	let len = reader.remain();
	from_bytes( (0 .. len).map(|_| reader.read_u8().unwrap()) )
}