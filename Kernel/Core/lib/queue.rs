// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/queue.rs
//! A FIFO queue type
//!
//! Current implementation is a linked list, but could be backed to Vec if required.
use prelude::*;
use lib::{OptPtr,OptMutPtr};

/// A basic linked-list queue
pub struct Queue<T>
{
	#[doc(hidden)]
	pub head: OptPtr<QueueEnt<T>>,
	#[doc(hidden)]
	pub tail: OptMutPtr<QueueEnt<T>>,
}

unsafe impl<T: Sync> ::core::marker::Sync for Queue<T> {}
unsafe impl<T: Send> ::core::marker::Send for Queue<T> {}

/// Initialise a queue within a `static`
macro_rules! queue_init{ () => ($crate::lib::queue::Queue{head: $crate::lib::OptPtr(0 as *const _),tail: $crate::lib::OptMutPtr(0 as *mut _)}) }

#[doc(hidden)]
pub struct QueueEnt<T>
{
	next: OptPtr<QueueEnt<T>>,
	value: T
}

/// Immutable iterator
pub struct Items<'s, T: 's>
{
	cur_item: Option<&'s QueueEnt<T>>,
}
/// Mutable iterator
pub struct ItemsMut<'s, T: 's>
{
	cur_item: Option<&'s mut QueueEnt<T>>,
}

impl<T> Queue<T>
{
	/// Construct a new empty queue
	pub fn new() -> Queue<T> {
		queue_init!()	// I'm lazy
	}
	/// Add an item to the end of the queue
	pub fn push(&mut self, value: T)
	{
		unsafe
		{
			let qe_ptr = ::memory::heap::alloc( QueueEnt {
				next: OptPtr(0 as *const _),
				value: value,
				} );
			log_trace!("Pushing {:?}", qe_ptr);
			
			if self.head.is_some()
			{
				assert!( self.tail.is_some() );
				let r = self.tail.as_ref().unwrap();
				assert!( r.next.is_none() );
				r.next = OptPtr(qe_ptr as *const _);
			}
			else
			{
				self.head = OptPtr(qe_ptr as *const _);
			}
			self.tail = OptMutPtr(qe_ptr);
		}
	}
	/// Remove an item from the front
	pub fn pop(&mut self) -> ::core::option::Option<T>
	{
		if self.head.is_none() {
			return None;
		}
		
		unsafe
		{
			let qe_ptr = self.head.unwrap() as *mut QueueEnt<T>;
			self.head = ::core::ptr::read( &(*qe_ptr).next );
			if self.head.is_none() {
				self.tail = OptMutPtr(0 as *mut _);
			}
			
			let rv = ::core::ptr::read( &(*qe_ptr).value );
			::memory::heap::dealloc(qe_ptr);
			Some(rv)
		}
	}
	
	/// Returns true if the queue is empty
	pub fn empty(&self) -> bool
	{
		self.head.is_none()
	}
	
	/// Obtain an immutable iterator to the queue's items
	pub fn iter<'s>(&'s self) -> Items<'s,T>
	{
		Items {
			cur_item: unsafe { self.head.as_ref() },
		}
	}
	
	/// Obtain a mutable iterator to the queue's items
	pub fn iter_mut<'s>(&'s mut self) -> ItemsMut<'s,T>
	{
		ItemsMut {
			cur_item: unsafe { self.head.as_mut_ref() },
		}
	}
}

impl<'s, T> IntoIterator for &'s Queue<T>
{
	type Item = &'s T;
	type IntoIter = Items<'s, T>;
	fn into_iter(self) -> Items<'s, T> {
		self.iter()
	}
}

impl<'s, T> Iterator for Items<'s,T>
{
	type Item = &'s T;
	fn next(&mut self) -> Option<&'s T>
	{
		match self.cur_item
		{
		Some(ptr) => {
			self.cur_item = unsafe { ptr.next.as_ref() };
			Some(&ptr.value)
			},
		None => None
		}
	}
}

impl<'s, T> Iterator for ItemsMut<'s,T>
{
	type Item = &'s mut T;
	fn next(&mut self) -> Option<&'s mut T>
	{
		match self.cur_item.take()
		{
		None => None,
		Some(ptr) => {
			self.cur_item = unsafe { ptr.next.as_mut_ref() };
			Some(&mut ptr.value)
			}
		}
	}
}

// vim: ft=rust

