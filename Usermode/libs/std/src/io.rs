
#![no_std]

#[macro_use]
extern crate core;

use core::iter::Iterator;
use core::slice::SliceExt;
use core::result::Result::{Ok,Err};

/// Shorthand result type
pub type Result<T> = ::core::result::Result<T,Error>;

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

