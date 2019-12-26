// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/byteorder.rs
//! A local version of the byteorder crates.io crate.

pub type Result<T> = ::std::result::Result<T,Error>;
#[derive(Debug)]
pub enum Error
{
	UnexpectedEOF,
	Io(::std::io::Error),
}
impl From<::std::io::Error> for Error {
	fn from(e: ::std::io::Error) -> Error {
		Error::Io(e)
	}
}

macro_rules! read_signed {
	($ty:ty, $bits:expr, $data:expr) => ({
		let rv = $data;
		if rv < 1<<($bits-1) {
			rv as $ty
		}
		else if rv == 1<<($bits-1) {
			<$ty>::min_value()
		}
		else {
			- ((!rv + 1) as $ty)
		}
	});
}
//macro_rules! write_signed {
//	($ty:ty, $bits:expr, $val:expr) => ({
//		let v = $val;
//		if v > 0 {
//			v as $ty
//		}
//		else if v == -( (1 << ($bits-1)) as $ty ) {
//			1 << ($bits-1)
//		}
//		else {
//			!(-v as $ty + 1)
//		}
//	});
//}

pub trait ByteOrder
{
	fn read_u16(buf: &[u8]) -> u16;
	fn read_u32(buf: &[u8]) -> u32;
	fn read_u64(buf: &[u8]) -> u64;
	fn read_uint(buf: &[u8], nbytes: usize) -> u64;
	fn write_u16(buf: &mut [u8], n: u16);
	fn write_u32(buf: &mut [u8], n: u32);
	fn write_u64(buf: &mut [u8], n: u64);

	fn read_i16(buf: &[u8]) -> i16 {
		read_signed!(i16, 16, Self::read_u16(buf))
	}
	fn read_i32(buf: &[u8]) -> i32 {
		read_signed!(i32, 32, Self::read_u32(buf))
	}
	fn read_i64(buf: &[u8]) -> i64 {
		read_signed!(i64, 64, Self::read_u64(buf))
	}
	fn read_int(buf: &[u8], nbytes: usize) -> i64 {
		read_signed!(i64, 64, Self::read_uint(buf, nbytes))
	}
	//fn read_f32(buf: &[u8]) -> f32 { ... }
	//fn read_f64(buf: &[u8]) -> f64 { ... }
	//fn write_i16(buf: &mut [u8], n: i16) { Self::write_u16(buf, write_signed!(u16, 16, n)) }
	//fn write_i32(buf: &mut [u8], n: i32) { Self::write_u32(buf, write_signed!(u32, 32, n)) }
	//fn write_i64(buf: &mut [u8], n: i64) { Self::write_u64(buf, write_signed!(u64, 64, n)) }
	//fn write_f32(buf: &mut [u8], n: f32) { ... }
	//fn write_f64(buf: &mut [u8], n: f64) { ... }
}

pub struct LittleEndian;
impl ByteOrder for LittleEndian
{
	fn read_u16(buf: &[u8]) -> u16 {
		(buf[0] as u16) | (buf[1] as u16) << 8
	}
	fn read_u32(buf: &[u8]) -> u32 {
		(buf[0] as u32) | (buf[1] as u32) << 8  | (buf[2] as u32) << 16 | (buf[3] as u32) << 24
	}
	fn read_u64(buf: &[u8]) -> u64 {
		Self::read_u32(&buf[0..4]) as u64 | (Self::read_u32(&buf[4..8]) as u64) << 32
	}
	fn read_uint(buf: &[u8], nbytes: usize) -> u64 {
		let mut rv = 0;
		for i in 0 .. nbytes {
			rv |= (buf[i] as u64) << (8*i);
		}
		rv
	}
	fn write_u16(buf: &mut [u8], n: u16) {
		buf[0] = ((n >> 0) & 0xFF) as u8;
		buf[1] = ((n >> 8) & 0xFF) as u8;
	}
	fn write_u32(buf: &mut [u8], n: u32) {
		buf[0] = ((n >>  0) & 0xFF) as u8;
		buf[1] = ((n >>  8) & 0xFF) as u8;
		buf[2] = ((n >> 16) & 0xFF) as u8;
		buf[3] = ((n >> 24) & 0xFF) as u8;
	}
	fn write_u64(buf: &mut [u8], n: u64) {
		Self::write_u32(&mut buf[0..4], ((n >>  0) & 0xFFFFFFFF) as u32);
		Self::write_u32(&mut buf[4..8], ((n >> 32) & 0xFFFFFFFF) as u32);
	}
}

pub struct BigEndian;
impl ByteOrder for BigEndian
{
	fn read_u16(buf: &[u8]) -> u16 {
		(buf[0] as u16) << 8 | (buf[1] as u16) << 0
	}
	fn read_u32(buf: &[u8]) -> u32 {
		(buf[0] as u32) << 24 | (buf[1] as u32) << 16 | (buf[2] as u32) << 8 | (buf[3] as u32) << 0
	}
	fn read_u64(buf: &[u8]) -> u64 {
		(Self::read_u32(&buf[0..4]) as u64) << 32 | (Self::read_u32(&buf[4..8]) as u64)
	}
	fn read_uint(buf: &[u8], nbytes: usize) -> u64 {
		let mut rv = 0;
		for i in 0 .. nbytes {
			rv |= (buf[i] as u64) << (8*(nbytes - 1 - i));
		}
		rv
	}
	fn write_u16(buf: &mut [u8], n: u16) {
		buf[0] = ((n >> 8) & 0xFF) as u8;
		buf[1] = ((n >> 0) & 0xFF) as u8;
	}
	fn write_u32(buf: &mut [u8], n: u32) {
		buf[0] = ((n >> 24) & 0xFF) as u8;
		buf[1] = ((n >> 16) & 0xFF) as u8;
		buf[2] = ((n >>  8) & 0xFF) as u8;
		buf[3] = ((n >>  0) & 0xFF) as u8;
	}
	fn write_u64(buf: &mut [u8], n: u64) {
		Self::write_u32(&mut buf[0..4], ((n >> 32) & 0xFFFFFFFF) as u32);
		Self::write_u32(&mut buf[4..8], ((n >>  0) & 0xFFFFFFFF) as u32);
	}
}

pub trait ReadBytesExt:
	::std::io::Read
{
	#[doc(hidden)]
	fn read_exact(&mut self, dst: &mut [u8]) -> Result<()> {
		if self.read(dst).map_err(|e| Error::from(e))? != dst.len() {
			Err( Error::UnexpectedEOF )
		}
		else {
			Ok( () )
		}
	}
	
	fn read_u8(&mut self) -> Result<u8> {
		let mut buf = [0; 1];
		self.read_exact(&mut buf)?;
		Ok( buf[0] )
	}
	fn read_i8(&mut self) -> Result<i8> {
		Ok( read_signed!(i8, 8, self.read_u8()?) )
	}
	fn read_u16<T: ByteOrder>(&mut self) -> Result<u16> {
		let mut buf = [0; 2];
		self.read_exact(&mut buf)?;
		Ok( T::read_u16(&buf) )
	}
	fn read_i16<T: ByteOrder>(&mut self) -> Result<i16> {
		let mut buf = [0; 2];
		self.read_exact(&mut buf)?;
		Ok( T::read_i16(&buf) )
	}
	fn read_u32<T: ByteOrder>(&mut self) -> Result<u32> {
		let mut buf = [0; 4];
		self.read_exact(&mut buf)?;
		Ok( T::read_u32(&buf) )
	}
	fn read_i32<T: ByteOrder>(&mut self) -> Result<i32> {
		let mut buf = [0; 4];
		self.read_exact(&mut buf)?;
		Ok( T::read_i32(&buf) )
	}
	fn read_u64<T: ByteOrder>(&mut self) -> Result<u64> {
		let mut buf = [0; 8];
		self.read_exact(&mut buf)?;
		Ok( T::read_u64(&buf) )
	}
	fn read_i64<T: ByteOrder>(&mut self) -> Result<i64> {
		let mut buf = [0; 8];
		self.read_exact(&mut buf)?;
		Ok( T::read_i64(&buf) )
	}
	fn read_uint<T: ByteOrder>(&mut self, nbytes: usize) -> Result<u64> {
		assert!(nbytes <= 8);
		let mut buf = [0; 8];
		self.read_exact(&mut buf)?;
		Ok( T::read_uint(&buf, nbytes) )
	}
	fn read_int<T: ByteOrder>(&mut self, nbytes: usize) -> Result<i64> {
		assert!(nbytes <= 8);
		let mut buf = [0; 8];
		self.read_exact(&mut buf)?;
		Ok( T::read_int(&buf, nbytes) )
	}
	//fn read_f32<T: ByteOrder>(&mut self) -> Result<f32> {
	//}
	//fn read_f64<T: ByteOrder>(&mut self) -> Result<f64> {
	//}
}
impl<T: ::std::io::Read + ?Sized> ReadBytesExt for T {
}

