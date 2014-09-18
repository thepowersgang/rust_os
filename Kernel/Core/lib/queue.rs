//
//
//
#![macro_escape]
use _common::*;

pub struct Queue<T>
{
	pub head: ::core::option::Option<*mut QueueEnt<T>>,
	pub tail: ::core::option::Option<*mut QueueEnt<T>>,
}

struct QueueEnt<T>
{
	next: ::core::option::Option<*mut QueueEnt<T>>,
	value: T
}

impl<T> Queue<T>
{
	pub fn push(&mut self, value: T)
	{
		unsafe
		{
			let qe_ptr = ::memory::heap::alloc::<QueueEnt<T>>();
			(*qe_ptr).next = None;
			::core::mem::overwrite( &mut (*qe_ptr).value, value );
			
			if self.head.is_some()	
			{
				assert!(self.tail.is_some());
				assert!((*self.tail.unwrap()).next.is_none());
				(*self.tail.unwrap()).next = Some(qe_ptr);
			}
			else
			{
				self.head = Some(qe_ptr);
				self.tail = Some(qe_ptr);
			}
		}
	}
	pub fn pop(&mut self) -> ::core::option::Option<T>
	{
		if self.head.is_none() {
			return None;
		}
		
		unsafe
		{
			let qe_ptr = self.head.unwrap();
			self.head = (*qe_ptr).next;
			if self.head.is_none() {
				self.tail = None;
			}
			
			let rv = ::core::mem::replace(&mut (*qe_ptr).value, ::core::mem::zeroed());
			::memory::heap::deallocate(qe_ptr as *mut ());
			Some(rv)
		}
	}
	
	pub fn empty(&self) -> bool
	{
		self.head.is_none()
	}
}

macro_rules! queue_init( () => (Queue{head: ::core::option::None,tail: ::core::option::None}) )

// vim: ft=rust

