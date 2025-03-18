// Tifflin OS Usermode
// - By John Hodge (thePowersGang)
//
//! libstd's IO support
#![no_std]
use core::fmt;

#[macro_use]
extern crate macros;
extern crate syscalls;

extern crate alloc;

pub mod prelude {
	pub use super::{Read, Write, BufRead, Seek};
}
mod std {
	pub use core::convert;
}

mod buf_reader;

pub use buf_reader::BufReader;

/// Shorthand result type
pub type Result<T> = ::core::result::Result<T,Error>;

/// IO Error type
#[derive(Debug)]
pub struct Error(ErrorInner);
#[derive(Debug)]
enum ErrorInner
{
	Misc,
	//Interrupted,
	VFS(::syscalls::vfs::Error),
	Net(::syscalls::net::Error),
}
impl ::core::fmt::Display for Error {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		match self.0 {
		ErrorInner::Misc => f.write_str("Unknown/misc error"),
		ErrorInner::VFS(::syscalls::vfs::Error::FileNotFound) => f.write_str("File not found"),
		ErrorInner::VFS(::syscalls::vfs::Error::TypeError   ) => f.write_str("Incorrect file type for operation"),
		ErrorInner::VFS(::syscalls::vfs::Error::PermissionDenied) => f.write_str("Permission denied"),
		ErrorInner::VFS(::syscalls::vfs::Error::FileLocked) => f.write_str("File is locked"),
		ErrorInner::VFS(::syscalls::vfs::Error::MalformedPath) => f.write_str("Malformed path"),
		//ErrorInner::VFS(ref e) => write!(f, "Unknown VFS error {:?}", e),
		ErrorInner::Net(::syscalls::net::Error::AlreadyInUse) => f.write_str("Address already in use"),
		ErrorInner::Net(::syscalls::net::Error::InvalidValue) => f.write_str("Invalid parameter value"),
		ErrorInner::Net(::syscalls::net::Error::NoData) => f.write_str("No data available"),
		}
	}
}

impl_conv! {
	From<::syscalls::vfs::Error>(v) for Error {
		Error( ErrorInner::VFS(v) )
	}
	From<::syscalls::net::Error>(v) for Error {
		Error( ErrorInner::Net(v) )
	}
}

pub trait Read
{
	fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
}
impl<'a, T: 'a + Read> Read for &'a mut T {
	fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
		(**self).read(buf)
	}
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
			Ok(0) => return Err(Error(ErrorInner::Misc)/*::new(ErrorKind::WriteZero, "failed to write whole buffer")*/),
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
impl<'a, T: 'a + Seek> Seek for &'a mut T {
	fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
		(**self).seek(pos)
	}
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


impl Read for ::syscalls::vfs::File {
	fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
		Ok(try!( self.read(buf) ))
	}
}
impl Seek for ::syscalls::vfs::File {
	fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
		match pos
		{
		SeekFrom::Start(pos) => self.set_cursor(pos),
		SeekFrom::End(ofs) => {
			let pos = if ofs < 0 {
				self.get_size() - (-ofs) as u64
				} else {
				self.get_size() + ofs as u64
				};
			self.set_cursor(pos);
			},
		SeekFrom::Current(ofs) => {
			let pos = if ofs < 0 {
				self.get_cursor() - (-ofs) as u64
				} else {
				self.get_cursor() + ofs as u64
				};
			self.set_cursor(pos);
			},
		}
		Ok(self.get_cursor())
	}
}
