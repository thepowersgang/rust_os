
/// Chain of wrapping packet information, used for scatter-gather DMA
// TODO: Represent the lifetime of the components relative to the async root
// - Two lifetime parameters, one for inner and one for outer
pub struct SparsePacket<'a>
{
	head: &'a [u8],
	next: Option<&'a SparsePacket<'a>>,
}
impl<'a> SparsePacket<'a>
{
	pub fn new_root(data: &'a [u8]) -> SparsePacket<'a> {
		SparsePacket {
			head: data,
			next: None,
			}
	}
	pub fn new_chained(data: &'a [u8], next: &'a SparsePacket<'a>) -> SparsePacket<'a> {
		SparsePacket {
			head: data,
			next: Some(next),
			}
	}

	pub fn total_len(&self) -> usize {
		let mut s = self;
		let mut rv = 0;
		loop {
			rv += s.head.len();
			match s.next
			{
			None => break,
			Some(v) => s = v,
			}
		}
		rv
	}
}
impl<'a> IntoIterator for &'a SparsePacket<'a>
{
	type IntoIter = SparsePacketIter<'a>;
	type Item = &'a [u8];
	fn into_iter(self) -> SparsePacketIter<'a> {
		SparsePacketIter(Some(self))
	}
}
pub struct SparsePacketIter<'a>(Option<&'a SparsePacket<'a>>);
impl<'a> Iterator for SparsePacketIter<'a> {
	type Item = &'a [u8];
	fn next(&mut self) -> Option<Self::Item> {
		let p = match self.0
			{
			None => return None,
			Some(p) => p,
			};

		self.0 = p.next;
		Some(p.head)
	}
}

/// Handle to a packet in driver-owned memory
pub type PacketHandle<'a> = ::stack_dst::ValueA<dyn RxPacket + 'a, [usize; 8]>;
/// Trait representing a packet in driver-owned memory
pub trait RxPacket
{
	fn len(&self) -> usize;
	fn num_regions(&self) -> usize;
	fn get_region(&self, idx: usize) -> &[u8];
	fn get_slice(&self, range: ::core::ops::Range<usize>) -> Option<&[u8]>;
}

#[derive(Clone)]
pub struct PacketReader<'a> {
	pkt: &'a PacketHandle<'a>,
	ofs: usize,
}
impl<'a> PacketReader<'a> {
	pub(super) fn new(pkt: &'a PacketHandle<'a>) -> PacketReader<'a> {
		PacketReader {
			pkt: pkt,
			ofs: 0,
			}
	}
	pub fn remain(&self) -> usize {
		self.pkt.len() - self.ofs
	}
	pub fn read(&mut self, dst: &mut [u8]) -> Result<usize, ()> {
		// TODO: Should this be cached?
		let mut ofs = self.ofs;
		let mut r = 0;
		while ofs >= self.pkt.get_region(r).len() {
			ofs -= self.pkt.get_region(r).len();
			r += 1;
			if r == self.pkt.num_regions() {
				return Err( () );
			}
		}

		let mut wofs = 0;
		while wofs < dst.len() && self.ofs + wofs < self.pkt.len()
		{
			let rgn = self.pkt.get_region(r);
			let alen = rgn.len() - ofs;
			let rlen = dst.len() - wofs;
			let len = ::core::cmp::min(alen, rlen);

			dst[wofs..][..len].copy_from_slice( &rgn[ofs..][..len] );
			
			r += 1;
			ofs = 0;
			wofs += len;
		}

		self.ofs += wofs;
		Ok(wofs)
	}

	pub fn read_bytes<T: AsMut<[u8]>>(&mut self, mut b: T) -> Result<T, ()> {
		self.read(b.as_mut())?;
		Ok(b)
	}
	pub fn read_u8(&mut self) -> Result<u8, ()> {
		let mut b = [0];
		self.read(&mut b)?;
		Ok( b[0] )
	}
	pub fn read_u16n(&mut self) -> Result<u16, ()> {
		let mut b = [0,0];
		self.read(&mut b)?;
		Ok( (b[0] as u16) << 8 | (b[1] as u16) )
	}
	pub fn read_u32n(&mut self) -> Result<u32, ()> {
		let mut b = [0,0,0,0];
		self.read(&mut b)?;
		Ok( (b[0] as u32) << 24 + (b[1] as u32) << 16 | (b[2] as u32) << 8 | (b[3] as u32) )
	}
}