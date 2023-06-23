
// Takes a compression unit worth of data, and yields 4096 byte blocks from it
pub struct Decompressor<'a>(&'a [u8]);
impl<'a> Decompressor<'a>
{
	pub fn new(v: &'a [u8]) -> Self {
		//log_debug!("{:?}", ::kernel::logging::HexDump(v));
		Decompressor(v)
	}
	/// Decompress a block out of the stream
	pub fn get_block(&mut self, dst: Option<&mut [u8]>) -> Option<usize>
	{
		let dst = dst.unwrap_or(&mut []);
		let Some(hdr) = ::kernel::lib::split_off_front(&mut self.0, 2) else {
			return None;
			};
		let hdr = u16::from_le_bytes([hdr[0], hdr[1]]);
		let compressed_len = (hdr & 0xFFF) as usize + 1;
		let Some(src) = ::kernel::lib::split_off_front(&mut self.0, compressed_len) else {
			// TODO: Print an error?
			log_error!("Decompressor::get_block: MALFORMED: Unable to obtain all of compressed data ({compressed_len} > {})", self.0.len());
			return None;
			};
		if hdr & 0x8000 == 0 {
			//log_debug!("Uncompressed block {:#x}", compressed_len);
			// Uncompresed data, hopefully the length is 0x1000
			let len = usize::min(src.len(), dst.len());
			dst[..len].copy_from_slice(&src[..len]);
			Some(compressed_len)
		}
		else {
			let mut ofs = 0;
			//log_debug!("{} Compressed block {:?}", self.0.len(), ::kernel::logging::HexDump(src));
			// Compressed data, a sequence of token-groups preceded by a bitmap indicating the token classes
			let mut it = Tokens::new(src);
			while let Some(t) = it.next().expect("Malformed compressed data?")
			{
				match t
				{
				Token::Literal(b) => {
					if ofs < dst.len() {
						dst[ofs] = b;
					}
					ofs += 1;
					},
				Token::Lookback(dist_back, length) => {
					//log_debug!("Token::Lookback(-{}+{})", dist_back, length);
					if dist_back > ofs || ofs + length > 4096 {
						log_error!("Decompressor::get_block: MALFORMED: Lookback bad (-{}+{}, ofs={})",
							dist_back, length, ofs);
						return None;
					}
					if ofs < dst.len() {
						//log_debug!("{} += {}..{} - {:?}", ofs, ofs-dist_back, ofs-dist_back+length, ::kernel::logging::HexDump(&dst[ofs-dist_back..usize::min(ofs-dist_back+length,ofs)]));
						for _ in 0 .. length {
							if ofs < dst.len() {
								let v = dst[ofs - dist_back];
								dst[ofs] = v;
							}
							ofs += 1;
						}
					}
					else {
						ofs += length;
					}
					},
				}
			}
			Some(ofs)
		}
	}
}

struct Tokens<'a> {
	src: ::core::iter::Copied<::core::slice::Iter<'a, u8>>,
	bits: BitsIter,
	ofs: usize,
}
impl<'a> Tokens<'a> {
	pub fn new(src: &'a [u8]) -> Self {
		Tokens {
			src: src.iter().copied(),
			bits: BitsIter::empty(),
			ofs: 0,
		}
	}
	pub fn next(&mut self) -> Result<Option<Token>,()> {
		let is_lookback = if let Some(b) = self.bits.next() {
				b
			}
			else if let Some(b) = self.src.next() {
				self.bits = BitsIter::new(b);
				self.bits.next().unwrap()
			}
			else {
				return Ok(None)
			};
		Ok(Some(if is_lookback {
			let v1 = self.src.next().ok_or(())?;
			let v2 = self.src.next().ok_or(())?;
			let token = u16::from_le_bytes([v1, v2]);
			// The token is split into a negative distance and a size.
			// - This split changes depending on how far into the block we are.
			let dshift = {
				let mut dshift = 12;
				// TODO: `ntfsdoc.pdf` pp74 lists the following C algorithm
				// `for(i=clear_pos-1,dshift=12;i>=0x10;i>>=1){`
				// I'm not sure what `clear_pos` is - it's probably the write position, but there might be an off-by-one?
				let mut i = self.ofs.saturating_sub(1);
				while i >= 0x10 {
					dshift -= 1;
					i /= 2;
				}
				dshift
				};
			let dist_back = (token >> dshift) as usize + 1;	// A zero distance is useless.
			let length = (token & ((1 << dshift)-1)) as usize + 3;
			//log_debug!("{:#x} = +{}+{} (dshift={})", token, dist_back, length, dshift);

			self.ofs += length;
			Token::Lookback(dist_back, length)
		}
		else {
			self.ofs += 1;
			if let Some(b) = self.src.next() {
				Token::Literal(b)
			}
			else {
				return Ok(None);
			}
		}))
	}
}

enum Token {
	Literal(u8),
	Lookback(usize, usize),
}

struct BitsIter {
	val: u8,
	ofs: u8,
}
impl BitsIter {
	fn empty() -> BitsIter {
		BitsIter { val: 0, ofs: 8 }
	}
	fn new(v: u8) -> BitsIter {
		BitsIter { val: v, ofs: 0 }
	}
}
impl Iterator for BitsIter {
	type Item = bool;
	fn next(&mut self) -> Option<bool> {
		if self.ofs == 8 {
			None
		}
		else {
			self.ofs += 1;
			Some( (self.val >> (self.ofs-1)) & 1 != 0 )
		}
	}
}


#[cfg(test)]
mod tests {
	use super::Decompressor;
	#[test]
	fn decomp_0()
	{
		// Example from `ntfsdoc.pdf` (p75) - A block filled with 0x20 (space)
		let cdat = &[ 3,0xb0,  0x2, b' ', 0xFC,0x0F, ];
		let mut d = Decompressor::new(cdat);
		let mut dst = [0xCC; 4096];
		assert_eq!(d.get_block(Some(&mut dst)), Some(4096));
		assert_eq!(dst, [b' '; 4096]);
	}

	#[test]
	fn decomp_1()
	{
		// TODO: Get a compressed NTFS volume
		// "#include <ntfs.h>\n (-18,10)stdio(-17,4)"
		let cdat = &[
			9+9+10/*+3*/-1,0xB0,
			0x00, b'#',b'i',b'n',b'c', b'l',b'u',b'd',b'e',
			0x00, b' ',b'<',b'n',b't', b'f',b's',b'.',b'h',
			0x04, b'>',b'\n', 0x07,0x88/*(-18,10)*/, b's',b't',b'd',b'i',b'o',
			//0x01, 0x04,0x90/*(-17,4)*/,
			];
		let exp = b"#include <ntfs.h>\n#include <stdio";
		let mut d = Decompressor::new(cdat);
		let mut dst = [0xCC; 4096];
		assert_eq!(d.get_block(Some(&mut dst)), Some(exp.len()));
		assert_eq!(&dst[..exp.len()], exp);
	}
}
