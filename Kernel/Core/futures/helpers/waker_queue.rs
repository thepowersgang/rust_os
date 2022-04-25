//!
//! 

use ::core::task;

#[derive(Default)]
pub struct WakerQueue {
	val1: Option<task::Waker>,
	// Would LOVE to use VecDeque but it doesn't have a `const` constructor
	others: ::alloc::vec::Vec<task::Waker>,
}

impl WakerQueue
{
	pub const fn new() -> Self {
		WakerQueue { val1: None, others: ::alloc::vec::Vec::new() }
	}
	pub fn push(&mut self, v: &task::Waker) {
		// Compact if the `others` list only has one item, and the non-alloc slot has none
		if self.others.len() == 1 && self.val1.is_none() {
			self.val1 = self.others.pop();
		}
		// If the waker is already in the list, don't push
		for e in Iterator::chain( self.val1.as_ref().into_iter(), self.others.iter() ) {
			if e.will_wake(v) {
				return ;
			}
		}
		
		// If there's other items, push to the end of that list.
		if self.others.len() > 0 {
			self.others.push(v.clone());
		}
		else {
			match self.val1
			{
			None => { self.val1 = Some(v.clone()); },
			Some(ref e) if e.will_wake(v) => {},
			Some(_) => {
				self.others.push(v.clone());
				}
			}
		}
	}
	pub fn pop(&mut self) -> Option<task::Waker> {
		if let Some(v) = self.val1.take() {
			Some(v)
		}
		else if self.others.len() > 0 {//let Some(v) = {self.others.remove(0) {
			Some( self.others.remove(0) )
		}
		else {
			None
		}
	}
	/// Wake the oldest addition to this list
	pub fn wake_one(&mut self) -> bool {
		if let Some(w) = self.pop() {
			w.wake();
			true
		}
		else {
			false
		}
	}
	/// Wake all wakers in this list
	pub fn wake_all(&mut self) {
		while let Some(w) = self.pop() {
			w.wake();
		}
	}
}
