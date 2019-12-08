// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
// Modules/network/tcp-lib/rx_buffer.rs
//! TCP RX window buffer (sparse-ly populated ring buffer)
use kernel::prelude::*;

pub struct RxBuffer
{
	// Number of bytes in the buffer
	// Equal to `8 * data.len() / 9`
	size: usize,
	// Start of the first non-consumed byte
	read_pos: usize,
	// Data followed by bitmap
	data: Vec<u8>,
}
#[derive(Debug,PartialEq)]
pub enum InsertError
{
	/// Insufficient space to push this data
	NoSpace { avail: usize },
	/// The data already in the buffer is different the newly-inserted data
	DataMismatch { offset: usize },
}
impl RxBuffer
{
	/// Create a new buffer with the specified initial size
	pub fn new(window_size: usize) -> RxBuffer
	{
		let mut rv = RxBuffer {
			size: 0, read_pos: 0, data: vec![],
			};
		rv.resize(window_size);
		rv
	}
	/// Insert data into the buffer a the specified offset
	///
	/// Returns Err with the amount of free space if not enough available
	pub fn insert(&mut self, offset: usize, data: &[u8]) -> Result<(), InsertError> {
		let space = self.size - offset;
		if space < data.len() {
			return Err(InsertError::NoSpace { avail: space });
		}
		for i in 0 .. data.len()
		{
			let ofs = (self.read_pos + offset + i) % self.size;
			if self.data[self.size..][ofs / 8] & 1 << (ofs % 8) != 0 {
				if self.data[ofs] != data[i] {
					return Err(InsertError::DataMismatch { offset: offset + i });
				}
			}
			self.data[self.size..][ofs / 8] |= 1 << (ofs % 8);
			self.data[ofs] = data[i];
		}
		Ok( () )
	}
	/// Remove data from the start of the buffer
	/// Returns the number of bytes read
	pub fn take(&mut self, buf: &mut [u8]) -> usize {
		let out_len = ::core::cmp::min( buf.len(), self.valid_len() );
		for i in 0 .. out_len
		{
			buf[i] = self.data[self.read_pos];
			self.data[self.size..][self.read_pos / 8] &= !(1 << (self.read_pos % 8));
			self.read_pos += 1;
			if self.read_pos == self.size {
				self.read_pos = 0;
			}
		}
		out_len
	}
	pub fn valid_len(&self) -> usize
	{
		// Number of valid bytes in the first partial bitmap entry
		let mut len = {
			let ofs = self.read_pos % 8;
			let v = self.data[self.size..][self.read_pos/8] >> ofs;
			(!v).trailing_zeros()
			};
		if len > 0
		{
			for i in 1 .. self.size / 8
			{
				let v = self.data[self.size ..][ (self.read_pos / 8 + i) % (self.size/8) ];
				if v != 0xFF
				{
					len += (!v).trailing_zeros();
					break;
				}
				else
				{
					len += 8;
				}
			}
			// NOTE: There's an edge case where if the buffer is 100% full, it won't return that (if the read position is unaligned)
			// But that isn't a critical problem.
		}
		len as usize
	}
	/// Resize the buffer
	pub fn resize(&mut self, new_size: usize) {
		self.compact();
		assert!(self.read_pos == 0);
		if new_size > self.size {
			// Resize underlying vector
			self.data.resize(new_size + (new_size + 7) / 8, 0u8);
			// Copy/move the bitmap up
			self.data[self.size ..].rotate_right( (new_size - self.size) / 8 );
		}
		else {
			// Move the bitmap down
			self.data[new_size ..].rotate_left( (self.size - new_size) / 8 );
			self.data.truncate( new_size + (new_size + 7) / 8 );
		}
		self.size = new_size;
	}
	/// Remove all data
	//pub fn clear(&mut self)
	//{
	//	self.read_pos = 0;
	//	for v in &mut self.data[self.size ..]
	//	{
	//		*v = 0;
	//	}
	//}
	/// Compact the current state so read_pos=0
	fn compact(&mut self)
	{
		if self.read_pos != 0
		{
			// Rotate data
			self.data[..self.size].rotate_left( self.read_pos );
			// Bitmap:
			// Step 1: Octet align
			let bitofs = self.read_pos % 8;
			if bitofs > 0
			{
				let bitmap_rgn = &mut self.data[self.size ..];
				let mut last_val = bitmap_rgn[0];
				for p in bitmap_rgn.iter_mut().rev()
				{
					let v = (last_val << (8 - bitofs)) | (*p >> bitofs);
					last_val = ::core::mem::replace(p, v);
				}
			}
			// Step 2: shift bytes down
			self.data[self.size ..].rotate_left( self.read_pos / 8 );
		}
	}
}
impl ::core::fmt::Debug for RxBuffer
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		for i in 0 .. self.size
		{
			f.write_str( if i == self.read_pos { "|" } else { " " })?;
			if self.data[self.size ..][ i / 8 ] & 1 << (i % 8) == 0 {
				f.write_str("--")?;
			}
			else {
				write!(f, "{:02x}", self.data[i])?;
			}
		}
		Ok( () )
	}
}

#[test]
// Insert and remove data
fn basic_use()
{
	let mut buf = RxBuffer::new(16);
	buf.insert(0, b"Hello World").expect("Insert 0");
	assert_eq!(buf.valid_len(), 5+1+5);
	{
		let mut b = [0; 5+1+5];
		assert_eq!( buf.take(&mut b), b.len() );
		assert_eq!(&b, b"Hello World");
	}
}
#[test]
// Insert sparse data, and cover overlap
fn merging()
{
	let mut buf = RxBuffer::new(16);
	buf.insert(1, b"b").expect("Insert 1");
	assert_eq!(buf.valid_len(), 0);
	buf.insert(0, b"a").expect("Insert 2");
	assert_eq!(buf.valid_len(), 2);

	buf.insert(0, b"ab").expect("Insert 3");
	buf.insert(2, b"0123456789").expect("Insert 4");
	{
		let mut b = [0; 12];
		assert_eq!( buf.take(&mut b), b.len() );
		assert_eq!(&b, b"ab0123456789");
		assert_eq!(buf.valid_len(), 0);
	}
}

#[test]
// Try to insert some over-sized data
fn oversize()
{
	let mut buf = RxBuffer::new(16);
	buf.insert(0, b"0123456789").expect("Insert 4");
	assert_eq!(buf.valid_len(), 10);
	assert_eq!( buf.insert(10, b"0123456789"), Err(InsertError::NoSpace { avail: 6 }));
	assert_eq!(buf.valid_len(), 10);
	{
		let mut b = [0; 10];
		assert_eq!( buf.take(&mut b), b.len() );
		assert_eq!(&b, b"0123456789");
	}
	assert_eq!(buf.valid_len(), 0);

	buf.insert(0, b"0123456789").expect("Insert 5");
	//println!("{:?}", buf);
	{
		let mut b = [0; 10];
		assert_eq!( buf.take(&mut b), b.len() );
		assert_eq!(&b, b"0123456789");
	}
}

#[test]
// Check wrapping behavior
fn wrapping()
{
	let mut buf = RxBuffer::new(16);
	// Insert 8 bytes, then remove them
	buf.insert(0, &[0; 8]);
	//println!("PI {:?}", buf);
	{ let mut b = [0; 8]; buf.take(&mut b); }
	//println!("PT {:?}", buf);
	assert_eq!(buf.valid_len(), 0);
	buf.insert(0, &[0xFF; 12]);
	//println!("PI2 {:?}", buf);
	assert_eq!(buf.valid_len(), 12);
	{ let mut b = [0; 12]; buf.take(&mut b); assert_eq!(b, [0xFF; 12]); }
}

