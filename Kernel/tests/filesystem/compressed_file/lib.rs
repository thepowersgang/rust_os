use std::convert::TryInto;
use ::std::io::{Read,Seek,SeekFrom};

pub const MAGIC: [u8; 4] = *b"CF\x0A\x7F";

pub struct Header
{
	pub magic: [u8; 4],
	pub block_size: u32,
	pub file_size: u64,
}
impl Header {
	pub fn from_bytes(buf: &[u8; 16]) -> Self {
		let mut h = &buf[..];
		fn split_off_front<'a, const N: usize>(v: &mut &'a [u8]) -> &'a [u8; N] {
			let (rv, a) = v.split_at(N);
			*v = a;
			rv.try_into().unwrap()
		}
		let magic: [u8; 4] = *split_off_front(&mut h);
		let block_size = split_off_front(&mut h);
		let file_size = split_off_front(&mut h);
		Header {
			magic,
			block_size: u32::from_le_bytes(*block_size),
			file_size: u64::from_le_bytes(*file_size),
		}
	}
	pub fn to_bytes(&self) -> [u8; 16] {
		let magic = self.magic;
		let block_size = self.block_size.to_le_bytes();
		let file_size = self.file_size.to_le_bytes();
		[
			magic[0],magic[1],magic[2],magic[3],
			block_size[0],block_size[1],block_size[2],block_size[3],
			file_size[0],file_size[1],file_size[2],file_size[3],
			file_size[4],file_size[5],file_size[6],file_size[7],
		]
	}
}

pub struct Reader
{
	/// File size as reported by the file header
	file_size: u64,
	/// Currently seeked position, allows `SeekFrom::End` to be efficient
	pending_seek: Option<u64>,
	/// Compression block size
	chunk_size: u32,
	/// Current compression block index
	cur_chunk: usize,
	/// Curent offset in the current compression block
	cur_chunk_ofs: u32,

	cur_decoder: ::flate2::read::ZlibDecoder<ReaderInner>,

	/// Cached start locations and sizes of compression blocks
	chunk_offsets: Vec<(u32,u64)>,
}
impl Reader
{
	pub fn new(mut fp: ::std::fs::File) -> ::std::io::Result<Reader>
	{
		let (block_size, file_size) = {
			let mut header_raw = [0; 16];
			if fp.read(&mut header_raw)? != header_raw.len() {
				return Err(::std::io::Error::new(::std::io::ErrorKind::InvalidData, "Header truncated"));
			}
			let h = Header::from_bytes(&header_raw);
			if h.magic != MAGIC {
				return Err(::std::io::Error::new(::std::io::ErrorKind::InvalidData, "Magic number mismatch"));
			}
			(h.block_size, h.file_size)
			};
		let mut rv = Reader {
			chunk_size: block_size,
			file_size,
			pending_seek: None,
			cur_chunk_ofs: 0,
			cur_chunk: 0,
			chunk_offsets: vec![],
			cur_decoder: ::flate2::read::ZlibDecoder::new(ReaderInner::Some(fp.take(0))),
		};
		rv.new_chunk()?;
		assert!(rv.cur_chunk == 0);
		Ok(rv)
	}
}

enum ReaderInner {
	None,
	Some(::std::io::Take<::std::fs::File>),
}
impl ReaderInner {
	pub fn take(&mut self) -> ::std::fs::File {
		match ::std::mem::replace(self, ReaderInner::None) {
		ReaderInner::None => panic!(),
		ReaderInner::Some(v) => v.into_inner(),
		}
	}
}
impl ::std::io::Read for ReaderInner {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		match self {
		ReaderInner::None => Ok(0),
		ReaderInner::Some(take) => take.read(buf),
		}
	}
}

impl Reader
{
	fn new_chunk(&mut self) -> ::std::io::Result<()>
	{
		if self.cur_chunk == 0 && self.cur_chunk_ofs == 0 {
		}
		else {
			self.cur_chunk += 1;
			assert!(self.cur_chunk == self.chunk_offsets.len());
			assert_eq!(self.cur_chunk_ofs, self.chunk_size);
		}

		let mut inner = self.cur_decoder.get_mut().take();
		let len = {
			let mut len_raw = [0; 4];
			inner.read_exact(&mut len_raw)?;
			u32::from_le_bytes(len_raw)
			};
		let ofs = inner.seek(SeekFrom::Current(0))?;
		//println!("new_chunk: {} = {:#x}+{:#x}", self.cur_chunk, ofs, len);
		self.chunk_offsets.push((len, ofs));
		
		self.cur_decoder.reset( ReaderInner::Some(inner.take(len as u64)) );
		self.cur_chunk_ofs = 0;
		Ok( () )
	}
	/// Seek to offset 0 in the specified chunk
	fn seek_chunk(&mut self, chunk_idx: usize) -> ::std::io::Result<()>
	{
		assert!(chunk_idx < self.chunk_offsets.len());
		let (len,ofs) = self.chunk_offsets[chunk_idx];
		//println!("seek_chunk: {} = {:#x}+{:#x}", chunk_idx, ofs, len);
		self.cur_chunk = chunk_idx;
		let mut inner = self.cur_decoder.get_mut().take();
		inner.seek(SeekFrom::Start(ofs))?;
		self.cur_decoder.reset( ReaderInner::Some(inner.take(len as u64)) );
		self.cur_chunk_ofs = 0;
		Ok( () )
	}
	/// Consume bytes from the current chunk
	fn seek_inner(&mut self, mut ofs: u32) -> ::std::io::Result<()>
	{
		self.cur_chunk_ofs += ofs;
		//println!("seek_inner: {:#x}", ofs);
		let mut buf = vec![0; 1024];
		while ofs >= buf.len() as u32 {
			ofs -= self.cur_decoder.read(&mut buf)? as u32;
		}
		self.cur_decoder.read(&mut buf[..ofs as usize])?;
		Ok( () )
	}
}

impl ::std::io::Read for Reader
{
	fn read(&mut self, mut buf: &mut [u8]) -> std::io::Result<usize> {
		if let Some(des_pos) = self.pending_seek.take()
		{
			//println!("SEEK {:#x} to {:#x}", (self.cur_chunk as u64 * self.chunk_size as u64) + self.cur_chunk_ofs as u64, des_pos);
			let (des_chunk,des_ofs) = (
				(des_pos / self.chunk_size as u64) as usize,
				(des_pos % self.chunk_size as u64) as u32,
				);
			if des_chunk == self.cur_chunk && des_ofs >= self.cur_chunk_ofs {
				// Read until target
				self.seek_inner(des_ofs - self.cur_chunk_ofs)?;
			}
			else if des_chunk <= self.cur_chunk || des_chunk < self.chunk_offsets.len() {
				// Reset state
				self.seek_chunk(des_chunk)?;
				self.seek_inner(des_ofs)?;
			}
			else {
				// Need to read through to the target offset
				// - If we're in the last chunk now, then complete it
				if self.cur_chunk + 1 == self.chunk_offsets.len() {
					self.seek_inner(self.chunk_size - self.cur_chunk_ofs)?;
					self.new_chunk()?;
				}
				self.seek_chunk(self.chunk_offsets.len() - 1)?;
				while self.cur_chunk + 1 <= des_chunk {
					self.seek_inner(self.chunk_size)?;
					self.new_chunk()?;
				}
				assert!(self.cur_chunk == des_chunk);
				self.seek_inner(des_ofs)?;
			}
			let cur_pos = (self.cur_chunk as u64 * self.chunk_size as u64) + self.cur_chunk_ofs as u64;
			assert!(cur_pos == des_pos);
		}
		//println!("READ @ {:#x}:{:#x} + {:#x}", self.cur_chunk, self.cur_chunk_ofs, buf.len());

		let mut total_read = 0;
		let mut space = self.chunk_size - self.cur_chunk_ofs;
		while buf.len() > space as usize {
			if space > 0 {
				let (pre,post) = buf.split_at_mut(space as usize);
				self.cur_decoder.read_exact(pre)?;
				self.cur_chunk_ofs += pre.len() as u32;
				total_read += pre.len();
				buf = post;
			}
			if self.cur_chunk as usize == self.chunk_offsets.len() - 1 {
				self.new_chunk()?;
			}
			else {
				self.seek_chunk(self.cur_chunk + 1)?;
				//assert!(self.chunk_offsets[self.cur_chunk as usize] == cur_ofs);
			}
			space = self.chunk_size - self.cur_chunk_ofs;
		}
		self.cur_decoder.read_exact(buf)?;
		self.cur_chunk_ofs += buf.len() as u32;
		total_read += buf.len();

		Ok(total_read)
	}
}

impl ::std::io::Seek for Reader
{
	fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
		let cur_pos = self.pending_seek.unwrap_or( (self.cur_chunk as u64 * self.chunk_size as u64) + self.cur_chunk_ofs as u64 );
		let des_pos = match pos {
			SeekFrom::Current(ofs) => cur_pos.checked_add_signed(ofs)
				.ok_or(std::io::Error::new(::std::io::ErrorKind::InvalidInput, "Seeking before 0"))?,
			SeekFrom::Start(ofs) => ofs,
			SeekFrom::End(ofs) => self.file_size.checked_add_signed(-ofs)
				.ok_or(std::io::Error::new(::std::io::ErrorKind::InvalidInput, "Seeking after end"))?,
			};
		if des_pos != cur_pos {
			self.pending_seek = Some(des_pos);
		}

		Ok(des_pos)
	}
}