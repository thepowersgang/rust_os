// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/queue.rs
//! A FIFO queue type
//!
//! Current implementation is a linked list, but could be backed to Vec if required.
use crate::prelude::*;

/// A basic linked-list queue
pub struct Queue<T>
{
	head: Option<QueueEntPtr<T>>,
	tail: Option<QueueTailPtr<T>>,
}

unsafe impl<T: Sync> ::core::marker::Sync for Queue<T> {}
unsafe impl<T: Send> ::core::marker::Send for Queue<T> {}

/// Initialise a queue within a `static`
macro_rules! queue_init{ () => ($crate::lib::queue::Queue::new()) }

struct QueueEntPtr<T>(Box<QueueEnt<T>>);
struct QueueTailPtr<T>(::core::ptr::NonNull<QueueEnt<T>>);

struct QueueEnt<T>
{
	next: Option<QueueEntPtr<T>>,
	value: T,
}

impl<T> Queue<T>
{
	/// Construct a new empty queue
	pub const fn new() -> Queue<T> {
		Queue{
			head: None,
			tail: None,
		}
	}
}
impl<T> Default for Queue<T>
{
	fn default() -> Self { Self::new() }
}
impl<T> Queue<T>
{
	/// Add an item to the end of the queue
	//pub fn push<U: ::core::ops::CoerceUnsized<T>>(&mut self, value: U)
	pub fn push(&mut self, value: T)
	{
		// SAFE: New QueueTailPtr should not outlive the object, tail does not alias
		unsafe
		{
			let mut qe_ptr = QueueEntPtr::new(value);
			let tail_ptr = qe_ptr.tail_ptr();
			
			if self.head.is_some()
			{
				// If the queue was non-empty, then push to tail
				match self.tail
				{
				Some(ref mut t) => t.get_mut().next = Some(qe_ptr),
				None => panic!("Tail pointer was None when head != None"),
				}
			}
			else
			{
				self.head = Some(qe_ptr);
			}
			self.tail = Some(tail_ptr);
		}
	}
	/// Remove an item from the front
	pub fn pop(&mut self) -> ::core::option::Option<T>
	{
		// Take head (will be updated if Some)
		match self.head.take()
		{
		Some(old_head) => {
			// Unwrap and destructure allocation
			let QueueEnt { next, value } = old_head.into_inner();
			self.head = next;
			// Update tail if head became None
			if self.head.is_none() {
				self.tail = None;
			}
			Some( value )
			},
		None => None,
		}
	}
	/// Obtain a borrow to the last element
	pub fn last(&self) -> Option<&T> {
		// SAFE: Lifetime and borrow are bound, aliasing will not occur
		self.tail.as_ref().map(|x| unsafe { &x.get().value })
	}
	
	/// Returns true if the queue is empty
	pub fn is_empty(&self) -> bool
	{
		self.head.is_none()
	}
	
	/// Obtain an immutable iterator to the queue's items
	pub fn iter<'s>(&'s self) -> Items<'s,T>
	{
		Items {
			cur_item: self.head.as_ref().map(|x| &**x),
		}
	}
	
	/// Obtain a mutable iterator to the queue's items
	pub fn iter_mut<'s>(&'s mut self) -> ItemsMut<'s,T>
	{
		ItemsMut {
			cur_item: self.head.as_mut().map(|x| &mut **x),
		}
	}

	/// Removes items that satisfy the filter function
	pub fn filter_out<F: Fn(&T)->bool>(&mut self, filter_fcn: F) {
		// 1. Check the head
		while match self.head {
			Some(ref r) => filter_fcn(&r.value),
			None => false,
			}
		{
			let oldhead: QueueEntPtr<T> = self.head.take().unwrap();
			self.head = oldhead.into_inner().next;
		}
		// 2. Now that the head is not being changed, filter the rest
		let newtail = match self.head
			{
			Some(ref mut head) => {
				// SAFE: No aliasing of this mut will happen
				let mut prev = unsafe { head.tail_ptr() };
				// Breakout happens when next is None
				loop {
					let next = {
						// SAFE: No aliasing present if the list is correctly constructed
						let prev_next_ref = unsafe { &mut prev.get_mut().next };
						// While the next item fails the filter
						while match *prev_next_ref {
							Some(ref r) => filter_fcn(&r.value),
							None => false,
							}
						{
							// Drop it
							let oldnext = prev_next_ref.take().unwrap();
							*prev_next_ref = oldnext.into_inner().next;
						}

						// Get a tail ref to the next item, and continue
						// SAFE: This returned pointer will be either dropped, or saved as the next tail
						prev_next_ref.as_mut().map(|x| unsafe { x.tail_ptr() })
						};
					match next {
					Some(n) => prev = n,
					None => break,
					}
				}
				Some(prev)
				},
			None => None,
			};
		self.tail = newtail;
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

impl<T> QueueEntPtr<T>
{
	fn new(val: T) -> QueueEntPtr<T> {
		QueueEntPtr(Box::new(QueueEnt { next: None, value: val }))
	}

	fn into_inner(self) -> QueueEnt<T> {
		*self.0
	}

	/// UNSAFE: Requires that the tail pointer not outlive this object
	unsafe fn tail_ptr(&mut self) -> QueueTailPtr<T> {
		QueueTailPtr(::core::ptr::NonNull::new_unchecked(&mut *self.0))
	}
}
impl<T> ::core::ops::Deref for QueueEntPtr<T> {
	type Target = QueueEnt<T>;
	fn deref(&self) -> &QueueEnt<T> { &self.0 }
}
impl<T> ::core::ops::DerefMut for QueueEntPtr<T> {
	fn deref_mut(&mut self) -> &mut QueueEnt<T> { &mut self.0 }
}
impl<T> QueueTailPtr<T>
{
	/// UNSAFE: Can cause aliasing if called while &mut-s to the last object are active
	unsafe fn get(&self) -> &QueueEnt<T> {
		self.0.as_ref()
	}
	/// UNSAFE: Can cause aliasing if called while &mut-s to the last object are active
	unsafe fn get_mut(&mut self) -> &mut QueueEnt<T> {
		self.0.as_mut()
	}
}

/// Immutable iterator
pub struct Items<'s, T: 's>
{
	cur_item: Option<&'s QueueEnt<T>>,
}

impl<'s, T> Iterator for Items<'s,T>
{
	type Item = &'s T;
	fn next(&mut self) -> Option<&'s T>
	{
		match self.cur_item
		{
		Some(ptr) => {
			self.cur_item = ptr.next.as_ref().map(|x| &**x);
			Some(&ptr.value)
			},
		None => None
		}
	}
}

/// Mutable iterator
pub struct ItemsMut<'s, T: 's>
{
	cur_item: Option<&'s mut QueueEnt<T>>,
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
			self.cur_item = ptr.next.as_mut().map(|x| &mut **x);
			Some(&mut ptr.value)
			}
		}
	}
}

// vim: ft=rust

