// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/rwlock.rs
//! Reader-writer lock
use core::cell::UnsafeCell;
use core::marker::{Send,Sync};
use core::ops;

/// Reader-writer lock
pub struct RwLock<T: Send+Sync>
{
	inner: ::sync::Spinlock<RwLockInner>,
	data: UnsafeCell<T>,
}
unsafe impl<T: Send+Sync> Sync for RwLock<T> { }
unsafe impl<T: Send+Sync> Send for RwLock<T> { }
struct RwLockInner
{
	reader_count: i32,
	reader_queue: ::threads::WaitQueue,
	writer_queue: ::threads::WaitQueue,
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

impl<T: Send+Sync> RwLock<T>
{
	/// Construct a new Read-write lock wrapping the passed data
	pub const fn new(data: T) -> RwLock<T>
	{
		RwLock {
			inner: ::sync::Spinlock::new(RwLockInner {
				reader_count: 0,
				reader_queue: ::threads::WaitQueue::new(),
				writer_queue: ::threads::WaitQueue::new(),
				}),
			data: UnsafeCell::new(data),
		}
	}
	
	/// Obtain a read handle to the lock
	pub fn read<'a>(&'a self) -> Read<'a, T> {
		let mut lh = self.inner.lock();
		if lh.reader_count < 0
		{
			// A writer is active, sleep until it's done
			log_trace!("RwLock::read() - Wait on writer active");
			//lh.reader_queue.wait(lh);
			waitqueue_wait_ext!(lh, .reader_queue);
		}
		else if lh.writer_queue.has_waiter()
		{
			// A writer is waiting, sleep until it's done
			log_trace!("RwLock::read() - Wait on writer acquire");
			//lh.reader_queue.wait(lh);
			waitqueue_wait_ext!(lh, .reader_queue);
		}
		else
		{
			// Increment reader count and return success
			lh.reader_count += 1;
		}
		return Read { _lock: self }
	}
	/// Obtain a write (unique) handle
	pub fn write<'a>(&'a self) -> Write<'a, T> {
		let mut lh = self.inner.lock();
		if lh.reader_count != 0
		{
			log_trace!("RwLock::write() - Wait on writer acquire");
			//lh.writer_queue.wait(lh);
			waitqueue_wait_ext!(lh, .writer_queue);
			// When woken, the reader count will still be -1
		}
		else
		{
			lh.reader_count = -1;
		}
		return Write { _lock: self }
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
			// Threads are still active
			log_trace!("Read::drop() - Readers active");
		}
		else if lh.writer_queue.has_waiter() {
			log_trace!("Read::drop() - Yielding to writer");
			assert!(lh.reader_count == 0);
			// There's a writer waiting, yeild to it
			lh.reader_count = -1;
			lh.writer_queue.wake_one();
			// - Woken writer takes logical ownership of the write handle
		}
		else {
			// Fully released!
			//log_trace!("Read::drop() - Released");
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
		if lh.writer_queue.has_waiter()
		{
			log_trace!("Write::drop() - Yielding to other writer");
			//lh.reader_count = -1;
			lh.writer_queue.wake_one()
			// - Woken writer takes logical ownership of the write handle
		}
		else if lh.reader_queue.has_waiter()
		{
			log_trace!("Write::drop() - Waking readers");
			lh.reader_count = 0;
			while lh.reader_queue.has_waiter() {
				lh.reader_count += 1;
				lh.reader_queue.wake_one();
			}
			// - Woken readers assume that the count has been incremented, and we did
		}
		else
		{
			// Fully released
			//log_trace!("Write::drop() - Released");
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
