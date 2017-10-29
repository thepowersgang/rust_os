
use alloc::vec::Vec;

const DEFAULT_BUF_SIZE: usize = 0x1000-32;

pub struct BufReader<R>
{
	inner: R,

	offset: usize,
	buf: Vec<u8>,
}

impl<R: ::Read> BufReader<R>
{
	pub fn new(inner: R) -> BufReader<R> {
		Self::with_capacity(DEFAULT_BUF_SIZE, inner)
	}
	pub fn with_capacity(cap: usize, inner: R) -> BufReader<R> {
		BufReader {
			inner: inner,
			offset: 0,
			buf: Vec::with_capacity(cap),
			}
	}

	pub fn get_ref(&self) -> &R {
		&self.inner
	}
	pub fn into_inner(self) -> R {
		self.inner
	}
}

impl<R: ::Read> ::Read for BufReader<R>
{
	fn read(&mut self, buf: &mut [u8]) -> super::Result<usize> {
		let mut offset = 0;
		if self.offset < self.buf.len() {
			let buf_len = ::core::cmp::min( buf.len(), self.buf.len() - self.offset );
			buf[..buf_len].copy_from_slice( &self.buf[self.offset..][..buf_len] );
			self.offset += buf_len;
			offset += buf_len;
		}

		if offset < buf.len()
		{
			assert!( self.offset == self.buf.len() );
			if buf.len() - offset >= self.buf.capacity() {
				offset += try!(self.inner.read(&mut buf[offset..]));
			}
			else {
				let cap = self.buf.capacity();
				self.buf.resize(cap, 0);
				let len = try!(self.inner.read(&mut self.buf));
				self.buf.truncate(len);
				self.offset = 0;

				let buf_len = ::core::cmp::min( buf.len() - offset, self.buf.len() );
				buf[offset..][..buf_len].copy_from_slice( &self.buf[..buf_len] );
				self.offset += buf_len;
				offset += buf_len;
			}
		}
		Ok( offset )
	}
}

