// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/rwlock.rs
//! Reader-writer lock
use core::cell::UnsafeCell;
use core::marker::{Send,Sync};
use core::ops;

macro_rules! trace_type {
	($t:ty) => { false };
	//($t:ty) => { type_name!(T) == "core::option::Option<objects::UserObject>" };
}

/// Reader-writer lock
pub struct RwLock<T>
{
	inner: crate::sync::Spinlock<RwLockInner>,
	data: UnsafeCell<T>,
}
unsafe impl<T: Send+Sync> Sync for RwLock<T> { }
unsafe impl<T: Send+Sync> Send for RwLock<T> { }
struct RwLockInner
{
	reader_count: i32,
	reader_queue: crate::threads::WaitQueue,
	writer_queue: crate::threads::WaitQueue,
}

/// Read-write lock - Read handle
pub struct Read<'a, T:Send+Sync+'a>
{
	_lock: &'a RwLock<T>,
}
/// Read-write lock - Write handle
pub struct Write<'a, T:Send+Sync+'a>
{
	_lock: &'a RwLock<T>,
}
/// Read-write lock - Upgraded read handle
pub struct ReadAsWrite<'a,T:'a+Send+Sync>
{
	_lock: &'a RwLock<T>,
}

impl<T> RwLock<T>
{
	/// Construct a new Read-write lock wrapping the passed data
	pub const fn new(data: T) -> RwLock<T>
	{
		RwLock {
			inner: crate::sync::Spinlock::new(RwLockInner {
				reader_count: 0,
				reader_queue: crate::threads::WaitQueue::new(),
				writer_queue: crate::threads::WaitQueue::new(),
				}),
			data: UnsafeCell::new(data),
		}
	}

	/// Obtain `&mut` to the contained data
	pub fn get_mut(&mut self) -> &mut T {
		// SAFE: Have exclusive access (`&mut self`)
		unsafe {
			&mut *self.data.get()
		}
	}
}
	
impl<T: Send+Sync> RwLock<T>
{
	/// Obtain a read handle to the lock
	pub fn read<'a>(&'a self) -> Read<'a, T> {
		let mut lh = self.inner.lock();
		if lh.reader_count < 0
		{
			// A writer is active, sleep until it's done
			log_trace!("RwLock<{}>::read({:p}) - Wait on writer release", type_name!(T), self);
			//lh.reader_queue.wait(lh);
			waitqueue_wait_ext!(lh, .reader_queue);
		}
		else if lh.writer_queue.has_waiter()
		{
			// A writer is waiting, sleep until it's done
			log_trace!("RwLock<{}>::read({:p}) - Wait on writer acquire", type_name!(T), self);
			//lh.reader_queue.wait(lh);
			waitqueue_wait_ext!(lh, .reader_queue);
		}
		else
		{
			// Increment reader count and return success
			if trace_type!(T) {
				log_trace!("RwLock<{}>::read({:p}) - ACQUIRED", type_name!(T), self);
			}
			lh.reader_count += 1;
		}
		return Read { _lock: self }
	}
	pub fn try_read<'a>(&'a self) -> Option<Read<'a, T>> {
		let mut lh = self.inner.lock();
		if lh.reader_count < 0
		{
			// A writer is active
			None
		}
		else if lh.writer_queue.has_waiter()
		{
			// A writer is waiting
			None
		}
		else
		{
			// Increment reader count and return success
			if trace_type!(T) {
				log_trace!("RwLock<{}>::read({:p}) - ACQUIRED", type_name!(T), self);
			}
			lh.reader_count += 1;
			Some(Read { _lock: self })
		}
	}
	/// Obtain a write (unique) handle
	pub fn write<'a>(&'a self) -> Write<'a, T> {
		let mut lh = self.inner.lock();
		if lh.reader_count != 0
		{
			log_trace!("RwLock<{}>::write({:p}) - Wait on {} release", type_name!(T), self, if lh.reader_count > 0 { "reader" } else { "writer" } );
			//lh.writer_queue.wait(lh);
			waitqueue_wait_ext!(lh, .writer_queue);
			// When woken, the reader count will still be -1
		}
		else
		{
			if trace_type!(T) {
				log_trace!("RwLock<{}>::write({:p}) - ACQUIRED", type_name!(T), self);
			}
			lh.reader_count = -1;
		}
		return Write { _lock: self }
	}
	pub fn try_write(&self) -> Option<Write<T>> {
		let mut lh = self.inner.lock();
		if lh.reader_count != 0
		{
			None
		}
		else
		{
			if trace_type!(T) {
				log_trace!("RwLock<{}>::write({:p}) - ACQUIRED", type_name!(T), self);
			}
			lh.reader_count = -1;
			Some(Write { _lock: self })
		}
	}
}

impl<T: Send+Sync + ::core::default::Default> ::core::default::Default for RwLock<T>
{
	fn default() -> RwLock<T> {
		RwLock::new(<T as ::core::default::Default>::default())
	}
}

// --------------------------------------------------------------------

impl<'a, T: Send+Sync> Read<'a, T>
{
	pub unsafe fn from_raw<'b>(p: &'b RwLock<T>) -> Read<'b, T> {
		Read {
			_lock: &p,
			}
	}
/*
	/// Upgrades a read handle to a write one (waiting for all other readers to go idle)
	// TODO: What happens if this wait collides with another, or a write wait
	pub fn upgrade_ref<'b>(this: &'b mut Self) -> ReadAsWrite<'b, T> {
		let mut lh = this._lock.inner.lock();
		if lh.reader_count == 1 {
			lh.reader_count = -1;
			ReadAsWrite { _lock: this._lock }
		}
		else {
			todo!("Read::upgrade_ref");
		}
	}
	/// Upgrades the current handle to a mutable handle if it can
	pub fn try_upgrade_ref<'b>(this: &'b mut Self) -> Option<ReadAsWrite<'b, T>> {
		let mut lh = this._lock.inner.lock();
		if lh.reader_count == 1 {
			lh.reader_count = -1;
			Some( ReadAsWrite { _lock: this._lock } )
		}
		else {
			None
		}
	}
*/
}
impl<'a, T: Send+Sync> ops::Drop for Read<'a, T>
{
	fn drop(&mut self) {
		let mut lh = self._lock.inner.lock();
		assert!(lh.reader_count > 0, "Dropping 'Read' for RwLock, but no read locks active");
		lh.reader_count -= 1;
		if lh.reader_count > 0 {
			// Reader threads are still active
			log_trace!("Read<{}>::drop({:p}) - Readers active", type_name!(T), self._lock);
		}
		else if let Some(tid) = lh.writer_queue.wake_one() {
			// There's a writer waiting, yeild to it
			log_trace!("Read<{}>::drop({:p}) - Yielding to writer (TID{})", type_name!(T), self._lock, tid);
			assert!(lh.reader_count == 0);
			lh.reader_count = -1;
			// - Woken writer takes logical ownership of the write handle
		}
		else {
			// Fully released!
			if trace_type!(T) {
				log_trace!("Read<{}>::drop({:p}) - RELEASED", type_name!(T), self._lock);
			}
		}
	}
}
impl<'a, T: Send+Sync> ops::Deref for Read<'a, T>
{
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: Read is fully aliasable
		unsafe { &*self._lock.data.get() }
	}
}

// --------------------------------------------------------------------

impl<'a, T: Send+Sync> ops::Drop for Write<'a, T>
{
	fn drop(&mut self)
	{
		let mut lh = self._lock.inner.lock();
		assert!(lh.reader_count == -1, "Dropping 'Write' for RwLock, but no write lock active (reader_count={})", lh.reader_count);
		if let Some(tid) = lh.writer_queue.wake_one()
		{
			log_trace!("Write<{}>::drop({:p}) - Yielding to other writer (TID{})", type_name!(T), self._lock, tid);
			// - Woken writer takes logical ownership of the write handle
		}
		else if lh.reader_queue.has_waiter()
		{
			log_trace!("Write<{}>::drop({:p}) - Waking readers", type_name!(T), self._lock);
			lh.reader_count = 0;
			while let Some(_) = lh.reader_queue.wake_one() {
				lh.reader_count += 1;
			}
			// - Woken readers assume that the count has been incremented, and we did
		}
		else
		{
			// Fully released
			if trace_type!(T) {
				log_trace!("Write<{}>::drop({:p}) - RELEASED", type_name!(T), self._lock);
			}
			lh.reader_count = 0;
		}
	}
}
impl<'a, T: Send+Sync> ops::Deref for Write<'a, T>
{
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: & means that & to inner is valid
		unsafe { &*self._lock.data.get() }
	}
}
impl<'a, T: Send+Sync> ops::DerefMut for Write<'a, T>
{
	fn deref_mut(&mut self) -> &mut T {
		// SAFE: &mut means that &mut to inner is valid
		unsafe { &mut *self._lock.data.get() }
	}
}

// --------------------------------------------------------------------
/*
impl<'a, T: Send+Sync> ops::Drop for ReadAsWrite<'a, T>
{
	fn drop(&mut self)
	{
		let mut lh = self._lock.inner.lock();
		assert!(lh.reader_count < 0, "Dropping 'ReadAsWrite' for RwLock, but no write lock active");
		// Restore to a single reader and continue
		lh.reader_count = 1;
	}
}
impl<'a, T: Send+Sync> ops::Deref for ReadAsWrite<'a, T>
{
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: Read is fully aliasable
		unsafe { &*self._lock.data.get() }
	}
}
impl<'a, T: Send+Sync> ops::DerefMut for ReadAsWrite<'a, T>
{
	fn deref_mut(&mut self) -> &mut T {
		// SAFE: &mut means that &mut to inner is valid
		unsafe { &mut *self._lock.data.get() }
	}
}
*/
