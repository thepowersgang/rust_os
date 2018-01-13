// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async-v3/buffer.rs
//! Asynchronous buffer handles

/// Buffer providing a location for read data (incoming / mutable)
pub struct ReadBuffer<'a> {
}
/// Buffer providing a location for data to be written (outgoing / immutable)
pub struct WriteBuffer<'a> {
}

pub struct ReadBufferHandle<'a> {
	// Needs to hold a borrow on the buffer
	buf: &'a ReadBuffer<'a>,
}
pub struct WriteBufferHandle<'a> {
}

impl ReadBuffer<'a>
{
	// UNSAFE: If this is leaked while borrowed, the borrow will access invalidated memory
	pub unsafe fn new_borrow(data: &mut [u8]) -> ReadBuffer
	{
		todo!("ReadBuffer::new_borrow");
	}
	pub fn new_user(data: FreezeMut<[u8]>) -> ReadBuffer<'static>
	{
		todo!("ReadBuffer::new_user");
	}
	pub fn new_owned(size: usize) -> ReadBuffer<'static>
	{
		todo!("ReadBuffer::new_owned");
	}

	pub fn borrow(&self) -> ReadBufferHandle
	{
		todo!("ReadBuffer::borrow");
	}
}

