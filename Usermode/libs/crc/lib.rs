
mod precalc;

pub struct Crc32(u32);

impl Crc32
{
	pub fn new() -> Crc32 {
		Crc32(!0)
	}

	pub fn update(&mut self, buf: &[u8])
	{
		for &b in buf
		{
			let idx = (self.0 ^ (b as u32)) & 0xFF;
			self.0 = precalc::CRC32_TABLE[ idx as usize ] ^ (self.0 >> 8);
		}
	}

	pub fn finalise(&self) -> u32
	{
		!self.0
	}
}

