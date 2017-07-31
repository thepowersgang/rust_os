// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/async-v3/mutex.rs
//! Asynchonous mutex
use prelude::*;
use lib::collections::VecDeque;
use core::ops;
use core::cell::UnsafeCell;

#[derive(Default)]
pub struct MutexInner
{
	/// List of threads waiting on this mutex
	sleep_queue: VecDeque<super::WaitHandle>,
	/// Current lock handle index (used to ensure that callers of `ack_lock` are the ones that were woken)
	cur_index: usize,
	/// The mutex is locked, but might not be acknowledged
	locked: bool,
	/// There is an active `Handle` to this mutex
	held: bool,
}
pub struct Mutex<T>
{
	inner: ::sync::Mutex<MutexInner>,
	data: UnsafeCell<T>,
}

impl<T> Mutex<T>
{
	/// Construct a new async mutex containing the passed value
	pub const fn new(v: T) -> Mutex<T> {
		Mutex {
			inner: ::sync::Mutex::new(MutexInner {
				cur_index: 0,
				sleep_queue: VecDeque::new(),
				locked: false,
				helf: false,
				}),
			data: UnsafeCell::new(v),
			}
	}
	
	/// Obtain mutable access to the mutex data (if there is unique access to the Mutex)
	pub fn get_mut(&mut self) -> &mut T {
		// SAFE: Unique access to the mutex
		unsafe { &mut *self.data.get() }
	}

	/// Asynchronously lock the mutex
	pub fn lock_async(&self, mut waiter: super::WaitHandle) {
		let mut lh = self.inner.lock();
		if !lh.locked {
			assert!( !lh.held, "Mutex not locked, but is still held" );
			lh.locked = true;
			// Uncontended. We now have the lock.
			// - Schedule a wake with the next ID
			waiter.wake(lh.cur_index);
		}
		else {
			// Contented (can't just outright acquire the lock)
			// - Push this waiter onto the list of waiting threads
			lh.sleep_queue.push_back(waiter)
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
		// TODO: Mangle the ID so callers can't easily predict it.
		lh.cur_index += 1;
		lh.held = true;

		Handle { lock: self }
	}
}


pub struct Handle<'a, T: 'a>
{
	lock: &'a Mutex<T>,
}
impl<'a, T: 'a> ops::Drop for Handle<'a, T> {
	fn drop(&mut self) {
		let mut lh = self.lock.inner.lock();
		lh.held = false;
		// If there's somebody waiting on the mutex, wake them
		if let Some(mut h) = lh.sleep_queue.pop_front() {
			// TODO: Make some indication of who is currently holding the mutex?
			h.wake( lh.cur_index );
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


