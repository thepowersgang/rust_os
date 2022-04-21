// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async/sequential_queue.rs
//! Asynchronous wait queue
//!
//! Waiters are woken in order (acknowledgement required)
//!
//! NOTE: This queue requires that all waiters eventually run their
//! completion handlers (to correctly yield to the next waiter)
use core::fmt;

pub enum Waiter<'a>
{
	Complete,
	Active {
		queue: &'a Source,
		idx: usize,
	},
}

/// A wait queue
///
/// Allows a list of threads to wait on a single object (e.g. a Mutex)
pub struct Source
{
	// TODO: Have a local SleepObjectRef to avoid malloc on single-wait case
	waiters: crate::sync::Mutex< crate::lib::Queue< (usize, crate::threads::SleepObjectRef) > >,

	flags: crate::sync::Spinlock<Flags>,
}
impl Default for Source {
	fn default() -> Self {
		Source::new()
	}
}
struct Flags
{
	next_alloc: usize,
	next_signalled: usize,
	cur_signalled: usize,

	// Ordering of the above:
	// `next_alloc` > `cur_signalled` >= `next_signalled`
}
impl Flags {
	const fn new() -> Flags {
		Flags {
			next_alloc: 1,
			next_signalled: 0,
			cur_signalled: 0
			}
	}
	fn alloc(&mut self) -> usize {
		let rv = self.next_alloc;
		self.next_alloc = self.next_alloc.wrapping_add(1);
		rv
	}
	fn signal_one(&mut self) -> Result<usize, bool> {
		if self.next_signalled == self.next_alloc.wrapping_sub(1)
		{
			Err(false)
		}
		else if self.cur_signalled == self.next_alloc
		{
			Err(true)
		}
		else
		{
			let rv = self.next_signalled;
			self.next_signalled = self.next_signalled.wrapping_add(1);
			Ok(rv)
		}
	}
}


impl Source
{
	/// Create a new queue source
	pub const fn new() -> Source
	{
		Source {
			waiters: crate::sync::Mutex::new(crate::lib::Queue::new()),
			flags: crate::sync::Spinlock::new(Flags::new()),
		}
	}
	
	/// Create a waiter for this queue
	pub fn wait_on<'a>(&'a self) -> Waiter
	{
		// Allocate a flag and return an active waiter for this flag
		let flag = self.flags.lock().alloc();
		Waiter::Active { queue: self, idx: flag }
	}
/*
	pub fn wait_upon(&self, waiter: &mut ::threads::SleepObject) {
		let mut wh = self.waiters.lock();
		wh.push( waiter.get_ref() );
	}
	pub fn clear_wait(&self, waiter: &mut ::threads::SleepObject) {
		self.waiters.lock().filter_out(|ent| ent.is_from(waiter));
	}
	*/
	
	/// Wake a single waiting thread
	pub fn wake_one(&self) -> bool
	{
		// Options:
		// - Queue is empty (nobody else to wake).
		//  > cur_signalled == next_signalled == next_allocate-1
		// - Someone has been signalled before, but hasn't acked it yet.
		//  > cur_signalled < next_signalled <= next_allocate-1
		// - Queue non-empty, and signal to be raised
		//  > cur_signalled == next_signalled < next_allocate-1
		
		match self.flags.lock().signal_one()
		{
		Ok(to_wake_idx) => {
			// Search the waiter list for whichever handle is next on the queue.
			let lh = self.waiters.lock();
			for &(idx, ref w) in &*lh
			{
				if idx == to_wake_idx {
					w.signal();
					return true;
				}
			}
			true
			},
		Err(_) => false,
		}
	}
}

impl<'a> fmt::Debug for Waiter<'a>
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "queue::Waiter")
	}
}
impl<'a> super::PrimitiveWaiter for Waiter<'a>
{
	fn is_complete(&self) -> bool {
		match self
		{
		&Waiter::Complete => true,
		_ => false,
		}
	}
	fn poll(&self) -> bool {
		match self
		{
		&Waiter::Complete => true,
		&Waiter::Active { queue, idx } => queue.flags.lock().next_signalled == idx,
		}
	}
	fn run_completion(&mut self) {
		match *self
		{
		Waiter::Complete => panic!("Completion run when already complete"),
		Waiter::Active { queue, idx } => {
			let mut lh = queue.flags.lock();
			assert!(lh.cur_signalled == idx.wrapping_sub(1), "");
			assert!(lh.next_signalled == idx, "");
			lh.cur_signalled = lh.cur_signalled.wrapping_add(1);
			}
		}
		*self = Waiter::Complete;
	}
	fn bind_signal(&mut self, sleeper: &mut crate::threads::SleepObject) -> bool {
		match *self
		{
		Waiter::Complete => {
			// Didn't register, should be polled and removed from the list.
			false
			},
		Waiter::Active { queue, idx } => {
			queue.waiters.lock().push( (idx, sleeper.get_ref()) );
			! self.poll()
			}
		}
	}
	fn unbind_signal(&mut self) {
		match *self
		{
		Waiter::Complete => {},
		Waiter::Active { queue, idx } => {
			queue.waiters.lock().filter_out(|x| x.0 == idx);
			}
		}
	}
}

