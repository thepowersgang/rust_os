// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async-v3/mutex.rs
//! Asynchonous mutex
#[allow(unused_imports)]
use crate::prelude::*;
use crate::lib::collections::VecDeque;
use core::ops;
use core::cell::UnsafeCell;

#[derive(Default)]
pub struct MutexInner
{
	/// List of threads waiting on this mutex
	sleep_queue: VecDeque<super::ObjectHandle>,
	/// Current lock handle index (used to ensure that callers of `ack_lock` are the ones that were woken)
	cur_index: usize,
	/// The mutex is locked, but might not be acknowledged
	locked: bool,
	/// There is an active `Handle` to this mutex
	held: bool,
}
pub struct Mutex<T>
{
	inner: crate::sync::Mutex<MutexInner>,
	data: UnsafeCell<T>,
}

impl<T> Mutex<T>
{
	/// Construct a new async mutex containing the passed value
	pub const fn new(v: T) -> Mutex<T> {
		Mutex {
			inner: crate::sync::Mutex::new(MutexInner {
				cur_index: 0,
				sleep_queue: VecDeque::new(),
				locked: false,
				held: false,
				}),
			data: UnsafeCell::new(v),
			}
	}
	
	/// Obtain mutable access to the mutex data (if there is unique access to the Mutex)
	pub fn get_mut(&mut self) -> &mut T {
		// SAFE: Unique access to the mutex
		unsafe { &mut *self.data.get() }
	}

	pub fn try_lock(&self) -> Option<Handle<T>> {
		let mut lh = self.inner.lock();
		if !lh.locked {
			assert!( !lh.held, "Mutex not locked, but is still held" );
			lh.locked = true;
			lh.held = true;
			Some(Handle { lock: self })
		}
		else {
			None
		}
	}

	/// Asynchronously lock the mutex
	/// 
	/// This signals the current layer with a "handle" to the mutex (to be passed to `ack_lock`)
	pub fn lock_async(&self, object: super::ObjectHandle, _stack: super::StackPush) {
		let mut lh = self.inner.lock();
		if !lh.locked {
			assert!( !lh.held, "Mutex not locked, but is still held" );
			lh.locked = true;
			// Uncontended. We now have the lock.
			// - Schedule a wake with the next ID
			object.signal(lh.cur_index);
		}
		else {
			// Contented (can't just outright acquire the lock)
			// - Push this waiter onto the list of waiting threads
			lh.sleep_queue.push_back(object)
		}
	}

	/// Acquire a mutex lock using an index
	pub fn ack_lock(&self, index: usize) -> Handle<T> {
		let mut lh = self.inner.lock();
		assert!( !lh.held,
			"Attmpeting to acquire an async mutex which is already held" );
		assert!( lh.locked,
			"Attmpeting to acquire an async mutex which isn't locked" );
		assert_eq!(lh.cur_index, index,
			"Attempting to acknowledge an async mutex acquire using a mismatched index - {} != cur {}", index, lh.cur_index);
		// TODO: Mangle the ID so callers can't easily predict it? Or should this method be unsafe to indicate that if you
		// fudge the ID, it's your own fault?.
		lh.cur_index += 1;
		lh.held = true;

		Handle { lock: self }
	}
}

/// Handle to the mutex, dereferences to the inner `T`
pub struct Handle<'a, T: 'a>
{
	lock: &'a Mutex<T>,
}
impl<'a, T: 'a> ops::Drop for Handle<'a, T> {
	fn drop(&mut self) {
		let mut lh = self.lock.inner.lock();
		lh.held = false;
		// If there's somebody waiting on the mutex, wake them
		if let Some(h) = lh.sleep_queue.pop_front() {
			// TODO: Make some indication of who is currently holding the mutex?
			h.signal( lh.cur_index );
		}
		else {
			lh.locked = false;
		}
	}
}
impl<'a, T: 'a> ops::Deref for Handle<'a, T> {
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: Lock is locked
		unsafe { &*self.lock.data.get() }
	}
}
impl<'a, T: 'a> ops::DerefMut for Handle<'a, T> {
	fn deref_mut(&mut self) -> &mut T {
		// SAFE: Lock is locked
		unsafe { &mut *self.lock.data.get() }
	}
}


