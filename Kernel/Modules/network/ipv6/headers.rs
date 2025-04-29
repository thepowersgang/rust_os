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
			source: Address::from_reader(reader)?,
			destination: Address::from_reader(reader)?,
			})
	}
}

pub trait Opt: Sized {
	fn from_value(code: u8, reader: OptReader) -> Option<Self>;
	fn unknown(t: UnknownOptTy, code: u8, data: [u8; 14]) -> Self;
}
pub struct OptReader<'a,'b> {
	reader: &'a mut crate::nic::PacketReader<'b>,
	len: usize,
}
impl<'a,'b> OptReader<'a, 'b> {
	pub fn read_bytes_pad<A: AsMut<[u8]>+Default>(&mut self) -> Option<A> {
		let mut rv = A::default();
		let buf = rv.as_mut();
		let len = buf.len().min(self.len);
		match self.reader.read(&mut buf[..len]) {
		Ok(n_read) => {
			self.len -= n_read;
			Some(rv)
		},
		Err(()) => None,
		}
	}
	pub fn read_u32(&mut self) -> Option<u32> {
		match self.reader.read_u32n() {
		Ok(v) => {
			self.len -= 4;
			Some(v)
		}
		Err(()) => None,
		}
	}
}
impl ::core::ops::Drop for OptReader<'_,'_> {
	fn drop(&mut self) {
		for _ in 0 .. self.len {
			let _ = self.reader.read_u8();
		}
	}
}
pub enum UnknownOptTy {
	/// Just ignore the option
	Skip,
	/// Silently discard the packet
	Discard,
	/// Discard the packet and send an error reply, even if multicast
	DiscardAndErrorAlways,
	/// Discard the packet ana send an error reply if NOT multicast
	DiscardAndError,
}

pub struct OptionsIter<'a,'b, T> {
	reader: &'a mut crate::nic::PacketReader<'b>,
	pd: ::core::marker::PhantomData<fn() -> T>,
}
impl<'a,'b, T> OptionsIter<'a,'b, T>
where
	T: Opt
{
	pub fn new(reader: &'a mut crate::nic::PacketReader<'b>) -> Self {
		OptionsIter { reader, pd: ::core::marker::PhantomData }
	}
}
impl<'a,'b, T> Iterator for OptionsIter<'a,'b, T>
where
	T: Opt
{
	type Item = T;
	
	fn next(&mut self) -> Option<Self::Item> {
		let Ok(code) = self.reader.read_u8() else { return None };
		if code == 0 {
			return self.next();
		}
		let Ok(len) = self.reader.read_u8() else { return None };
		let len = len as usize;
		// Multi-byte pad
		if code == 1 {
			for _ in 0 .. len {
				let _ = self.reader.read_u8();
			}
			return self.next();
		}
		match T::from_value(code, OptReader { reader: self.reader, len }) {
		Some(v) => Some(v),
		None => {
			let blob = {
				let mut buf = [0; 14];
				let len = len.min(buf.len());
				let _ = self.reader.read(&mut buf[..len]);
				buf
				};
			let t = match code >> 6 {
				0 => UnknownOptTy::Skip,
				1 => UnknownOptTy::Discard,
				2 => UnknownOptTy::DiscardAndErrorAlways,
				3 => UnknownOptTy::DiscardAndError,
				_ => unreachable!(),
				};
			Some(T::unknown(t, code, blob))
		}
		}
	}
}