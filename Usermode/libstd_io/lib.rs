// Tifflin OS Usermode
// - By John Hodge (thePowersGang)
//
// libstd_io
#![feature(no_std)]
#![feature(core_slice_ext,core_str_ext)]
#![no_std]
use core::fmt;

pub mod prelude {
	pub use super::{Read, Write, BufRead, Seek};
}

/// Shorthand result type
pub type Result<T> = ::core::result::Result<T,Error>;

/// IO Error type
#[derive(Debug)]
pub struct Error;

pub trait Read
{
	fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
}
pub trait BufRead: Read
{
}
pub trait Write
{
	fn write(&mut self, buf: &[u8]) -> Result<usize>;
	fn flush(&mut self) -> Result<()>;

	fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
		while !buf.is_empty() {
			match self.write(buf) {
			Ok(0) => return Err(Error/*::new(ErrorKind::WriteZero, "failed to write whole buffer")*/),
			Ok(n) => buf = &buf[n..],
			//Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
			Err(e) => return Err(e),
			}
		}
		Ok(())
	}
	fn write_fmt(&mut self, fmt: fmt::Arguments) -> Result<()> {
		// Create a shim which translates a Write to a fmt::Write and saves
		// off I/O errors. instead of discarding them
		struct Adaptor<'a, T: ?Sized + 'a> {
			inner: &'a mut T,
			error: Result<()>,
		}

		impl<'a, T: Write + ?Sized> fmt::Write for Adaptor<'a, T> {
			fn write_str(&mut self, s: &str) -> fmt::Result {
				match self.inner.write_all(s.as_bytes()) {
					Ok(()) => Ok(()),
					Err(e) => {
						self.error = Err(e);
						Err(fmt::Error)
					}
				}
			}
		}

		let mut output = Adaptor { inner: self, error: Ok(()) };
		match fmt::write(&mut output, fmt) {
			Ok(()) => Ok(()),
			Err(..) => output.error
		}
	}
	//fn by_ref(&mut self) -> &mut Self where Self: Sized { ... }
	//fn broadcast<W: Write>(self, other: W) -> Broadcast<Self, W> where Self: Sized { ... }
}

pub enum SeekFrom
{
	Start(u64),
	End(i64),
	Current(i64),
}
pub trait Seek
{
	fn seek(&mut self, pos: SeekFrom) -> Result<u64>;
}

/// Updates the slice as it reads
impl<'a> Read for &'a [u8]
{
	fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
		let ret = ::core::cmp::min( self.len(), buf.len() );
		
		for (d,s) in buf.iter_mut().zip( self.iter() ) {
			*d = *s;
		}
		
		*self = &self[ret ..];
		Ok(ret)
	}
}

