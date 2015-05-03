// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/ring_buf.rs
// - Ring buffer (fixed size)
//!
//! Provides a fixed-capacity ring buffer
use memory::heap::ArrayAlloc;
use core::option::Option::{self,Some,None};
use core::result::Result::{self,Ok,Err};

/// Fixed-size ring buffer type
pub struct RingBuf<T>
{
	data: ArrayAlloc<T>,
	start: usize,
	len: usize,
}

/*
Atomic ringbuf notes:
- Use semi atomicity (try_lock_cpu)
- four indexes
- write: try_lock, inc far len, write close len, set close=far, ELSE, inc far len, write old far
*/

impl<T> RingBuf<T>
{
	/// Create a new (empty) ring buffer
	pub fn new(capacity: usize) -> RingBuf<T> {
		RingBuf {
			data: ArrayAlloc::new( capacity ),
			start: 0,
			len: 0,
		}
	}

	fn int_get_idx(&self, idx: usize) -> usize {
		// idx == len valid for insertion
		assert!( idx <= self.len );
		(self.start + idx) % self.data.count()
	}

	/// Push an item to the end of the buffer
	pub fn push_back(&mut self, val: T) -> Result<(),T>
	{
		assert!(self.len <= self.data.count());
		if self.len == self.data.count()
		{
			Err(val)
		}
		else
		{
			unsafe {
				let idx = self.int_get_idx(self.len);
				::core::ptr::write( self.data.get_ptr_mut(idx), val );
				self.len += 1;
			}
			Ok( () )
		}
	}
	
	pub fn back_mut(&mut self) -> Option<&mut T>
	{
		if self.len == 0
		{
			None
		}
		else
		{
			let idx = self.int_get_idx(self.len-1);
			Some( unsafe { &mut *self.data.get_ptr_mut(idx) } )
		}
	}
	
	/// Pop an item from the front of the buffer
	pub fn pop_front(&mut self) -> Option<T>
	{
		if self.len == 0
		{
			None
		}
		else
		{
			unsafe {
				let idx = self.start;
				self.start = self.int_get_idx(1);
				self.len -= 1;
				Some( ::core::ptr::read( self.data.get_ptr(idx) ) )
			}
		}
	}
}
