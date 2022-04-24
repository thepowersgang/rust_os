// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/ring_buf.rs
// - Ring buffer (fixed size)
//!
//! Provides a fixed-capacity ring buffer
#[allow(unused_imports)]
use crate::prelude::*;
use crate::memory::heap::ArrayAlloc;
use core::sync::atomic::{AtomicUsize,Ordering};
use crate::sync::Spinlock;

/// Fixed-size ring buffer type
pub struct RingBuf<T>
{
	data: ArrayAlloc<T>,
	start: usize,
	len: usize,
}

/// A more expensive interior-mutable (semi)atomic ring buffer
///
/// This is semi-atomic in that it's IRQ-safe (handling the case where the protector
/// is held by the current CPU).
pub struct AtomicRingBuf<T>
{
	read_protector: Spinlock<()>,
	write_protector: Spinlock<()>,
	
	data: ArrayAlloc<T>,
	start: AtomicUsize,
	end: AtomicUsize,
}

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

	pub fn is_empty(&self) -> bool {
		self.len == 0
	}
	pub fn len(&self) -> usize {
		self.len
	}

	/// Obtain a contigious slice of data from this buffer
	pub fn get_slices(&mut self, range: ::core::ops::Range<usize>) -> (&[T], &[T]) {
		// SAFE: Correct pointer accesses to initialised data
		unsafe {
			assert!(range.start <= self.len);
			assert!(range.end <= self.len);
			let len = range.len();
			assert!(len <= self.len);
			let tail_space = self.data.count() - self.start;
			let head = self.len.saturating_sub(tail_space);
			if range.start < tail_space {
				if range.end < tail_space {
					let s1 = ::core::slice::from_raw_parts(self.data.get_ptr(self.start + range.start), len);
					let s2 = &[];
					(s1, s2)
				}
				else {
					let s1 = ::core::slice::from_raw_parts(self.data.get_ptr(self.start + range.start), tail_space);
					let s2 = ::core::slice::from_raw_parts(self.data.get_ptr(0), len - tail_space);
					(s1, s2)
				}
			}
			else {
				assert!(len < head);
				let s1 = ::core::slice::from_raw_parts(self.data.get_ptr(range.start - tail_space), len);
				let s2 = &[];
				(s1,s2)
			}
		}
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
			// SAFE: No valid data already there
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
			// SAFE: Pointer is valid, self is &mut
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
			// SAFE: No data effectively forotten
			unsafe {
				let idx = self.start;
				self.start = self.int_get_idx(1);
				self.len -= 1;
				Some( ::core::ptr::read( self.data.get_ptr(idx) ) )
			}
		}
	}
}

impl<T: Send> AtomicRingBuf<T>
{
	/// Create a new (empty) ring buffer
	pub fn new(capacity: usize) -> AtomicRingBuf<T> {
		AtomicRingBuf {
			write_protector: Spinlock::new( () ),
			read_protector: Spinlock::new( () ),
			data: ArrayAlloc::new( capacity ),
			start: AtomicUsize::new(0),
			end: AtomicUsize::new(0),
		}
	}
	
	//#[is_safe(irq)]	// Handles IRQ safety
	/// Pop an item from the ring buffer
	pub fn pop(&self) -> Option<T>
	{
		let _irql = crate::sync::hold_interrupts();
		let _lh = self.read_protector.lock();
		
		let idx = self.start.load(Ordering::Relaxed);
		let next_idx = (idx + 1) % self.data.count();
		if idx == self.end.load(Ordering::Relaxed) {
			None
		}
		else {
			// SAFE: Content of cell is effectively forgotten after read
			unsafe {
				let rv = ::core::ptr::read(&*self.data.get_ptr(idx));
				self.start.store(next_idx, Ordering::Relaxed);
				Some( rv )
			}
		}
	}
	
	//#[is_safe(irq)]	// Handles IRQ safety
	/// Push onto the end, returning Err(val) if full
	pub fn push(&self, val: T) -> Result<(),T>
	{
		let _irql = crate::sync::hold_interrupts();
		let _lh = self.write_protector.lock();
		
		let pos = self.end.load(Ordering::Relaxed);
		let next_pos = (pos + 1) % self.data.count();
		if next_pos == self.start.load(Ordering::Relaxed) {
			Err( val )
		}
		else {
			// SAFE: No valid data already there
			unsafe {
				::core::ptr::write(&mut *(self.data.get_ptr(pos) as *mut _), val);
				self.end.store(next_pos, Ordering::Relaxed);
			}
			Ok( () )
		}
	}
}

#[test]
fn test_ring_basic()
{
	let mut r = RingBuf::<i32>::new(3);
	r.push_back(1).expect("push_back");
	r.push_back(2).expect("push_back");
	r.push_back(3).expect("push_back");
	assert_eq!(r.push_back(4), Err(4));
	assert_eq!(r.pop_front(), Some(1));
	assert_eq!(r.pop_front(), Some(2));
	assert_eq!(r.pop_front(), Some(3));
	assert_eq!(r.pop_front(), None);

}
