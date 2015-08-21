// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/io.rs
/// A clone of ::std::io
#[allow(unused_imports)]
use prelude::*;

/// Shorthand result type
pub type Result<T> = ::core::result::Result<T,Error>;

/// IO Error type
#[derive(Debug)]
pub struct Error;

pub trait Read
{
	fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
}

/// Updates the slice as it reads
impl<'a> Read for &'a [u8]
{
	fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
		let ret = ::core::cmp::min( self.len(), buf.len() );
		
		for (d,s) in zip!( buf.iter_mut(), self.iter() ) {
			*d = *s;
		}
		
		*self = &self[ret ..];
		Ok(ret)
	}
}

