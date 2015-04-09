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

pub struct Read<'a, T:Send+Sync+'a>
{
	_lock: &'a RwLock<T>,
}
pub struct Write<'a, T:Send+Sync+'a>
{
	_lock: &'a RwLock<T>,
}

impl<T: Send+Sync> RwLock<T>
{
	/// Construct a new Read-write lock wrapping the passed data
	pub fn new(data: T) -> RwLock<T>
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
			panic!("TODO: RwLock wait for read");
		}
		else
		{
			lh.reader_count += 1;
		}
		return Read { _lock: self }
	}
	/// Obtain a write (unique) handle
	pub fn write<'a>(&'a self) -> Write<'a, T> {
		let mut lh = self.inner.lock();
		if lh.reader_count != 0
		{
			panic!("TODO: RwLock wait for write");
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

impl<'a, T: Send+Sync> ops::Drop for Read<'a, T>
{
	fn drop(&mut self) {
		let mut lh = self._lock.inner.lock();
		assert!(lh.reader_count > 0, "Dropping 'Read' for RwLock, but no read locks active");
		lh.reader_count -= 1;
		assert!( ! lh.reader_queue.has_waiter() );
		if lh.reader_count == 0 && lh.writer_queue.has_waiter()
		{
			panic!("TODO: RwLock release to writer");
		}
	}
}
impl<'a, T: Send+Sync> ops::Deref for Read<'a, T>
{
	type Target = T;
	fn deref(&self) -> &T {
		unsafe { &*self._lock.data.get() }
	}
}


impl<'a, T: Send+Sync> ops::Drop for Write<'a, T>
{
	fn drop(&mut self) {
		let mut lh = self._lock.inner.lock();
		assert!(lh.reader_count < 0, "Dropping 'Write' for RwLock, but no write lock active");
		lh.reader_count = 0;
		if lh.writer_queue.has_waiter()
		{
			panic!("TODO: RwLock pass to writer");
		}
		else if lh.reader_queue.has_waiter()
		{
			panic!("TODO: RwLock release to readers");
		}
		else
		{
			// Fully released
		}
	}
}
impl<'a, T: Send+Sync> ops::Deref for Write<'a, T>
{
	type Target = T;
	fn deref(&self) -> &T {
		unsafe { &*self._lock.data.get() }
	}
}
impl<'a, T: Send+Sync> ops::DerefMut for Write<'a, T>
{
	fn deref_mut(&mut self) -> &mut T {
		unsafe { &mut *self._lock.data.get() }
	}
}

