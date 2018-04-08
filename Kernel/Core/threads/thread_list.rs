// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads/threadlist.rs
//! Owned list of threads
use core::ptr::NonNull;

use super::{Thread,ThreadPtr};

/// Intrusive linked list of threads
pub struct ThreadList
{
	first: Option<ThreadPtr>,
	last: Option< NonNull<Thread> >,
}
unsafe impl Send for ThreadList {}

pub const THREADLIST_INIT: ThreadList = ThreadList {first: None, last: None};

impl ThreadList
{
	/// Returns true if the thread list is empty
	pub fn empty(&self) -> bool
	{
		self.first.is_none()
	}
	/// Remove a thread from the front of the list
	pub fn pop(&mut self) -> Option<ThreadPtr>
	{
		match self.first.take()
		{
		Some(mut t) => {
			self.first = t.next.take();
			if self.first.is_none() {
				self.last = None;
			}
			Some(t)
			},
		None => None
		}
	}
	/// Push a thread to the back
	pub fn push(&mut self, t: ThreadPtr)
	{
		//log_debug!("Pushing thread {:?}", t);
		assert!(t.next.is_none());
		// Save a pointer to the allocation (for list tail)
		let ptr = &*t as *const Thread as *mut Thread;
		// 2. Tack thread onto end
		if self.first.is_some()
		{
			assert!(self.last.is_some());
			// SAFE: WaitQueue should be locked (and nobody has any of the list items borrowed)
			unsafe {
				let last_ref: &mut Thread = self.last.as_mut().unwrap().as_mut();
				assert!(last_ref.next.is_none());
				last_ref.next = Some(t);
			}
		}
		else
		{
			assert!(self.last.is_none());
			self.first = Some(t);
		}
		// SAFE: ptr is non-zero
		self.last = Some(unsafe { NonNull::new_unchecked(ptr) });
	}
}

