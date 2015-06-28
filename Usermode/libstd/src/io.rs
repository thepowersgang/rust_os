
use core::iter::Iterator;
use core::slice::SliceExt;
use std::result::Result::{Ok,Err};

/// Shorthand result type
pub type Result<T> = ::std::result::Result<T,Error>;

/// IO Error type
#[derive(Debug)]
pub struct Error;

pub trait Read
{
	fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
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
		let ret = ::std::cmp::min( self.len(), buf.len() );
		
		for (d,s) in buf.iter_mut().zip( self.iter() ) {
			*d = *s;
		}
		
		*self = &self[ret ..];
		Ok(ret)
	}
}

impl Read for ::tifflin_syscalls::vfs::File {
	fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
		match self.read(buf)
		{
		Ok(v) => Ok(v),
		Err(v) => {
			panic!("VFS File read err: {:?}", v);
			},
		}
	}
}
impl Seek for ::tifflin_syscalls::vfs::File {
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

