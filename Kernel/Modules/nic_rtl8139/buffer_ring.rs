// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
//! Container for a ring buffer of pooled objects
use core::ops;

pub type BufferRing4<V> = BufferRing<[V; 4]>;

pub struct BufferRing<S: Storage>
{
	inner: ::kernel::sync::Spinlock<Inner>,
	data: ::core::cell::UnsafeCell<S>,
}
unsafe impl<S: Storage + Send> Send for BufferRing<S> {}
unsafe impl<S: Storage + Send> Sync for BufferRing<S> {}

#[derive(Default)]
struct Inner
{
	wait_queue: ::kernel::threads::WaitQueue,
	// Index of next free entry
	next_free: u16,
	// Index of first used entry. If equal to next_free, all are free.
	first_used: u16,
}

pub trait Storage
{
	type Inner;
	fn len() -> usize;
	unsafe fn get(&self, usize)->*mut Self::Inner;
}

impl<S: Storage> BufferRing<S>
{
	pub fn new(data: S) -> BufferRing<S> {
		BufferRing {
			inner: Default::default(),
			data: ::core::cell::UnsafeCell::new(data),
			}
	}
	
	pub fn acquire_wait(&self) -> Handle<S> {
		let mut lh = self.inner.lock();
		while (lh.next_free + 1) % S::len() as u16 == lh.first_used {
			waitqueue_wait_ext!(lh, .wait_queue);
			lh = self.inner.lock();
		}
		
		let idx = lh.next_free as usize;
		lh.next_free = (lh.next_free + 1) % S::len() as u16;
		
		Handle {
			bs: self,
			idx: idx,
			}
	}
	
	pub fn get_first_used(&self) -> Option<usize> {
		let lh = self.inner.lock();
		if lh.first_used != lh.next_free {
			Some(lh.first_used as usize)
		}
		else {
			None
		}
	}

	/// Release an object by index
	pub unsafe fn release(&self, index: usize) {
		let mut lh = self.inner.lock();
		assert_eq!(index, lh.first_used as usize);
		lh.first_used = (lh.first_used + 1) % S::len() as u16;
		
		if lh.wait_queue.has_waiter() {
			lh.wait_queue.wake_one();
		}
	}
}

pub struct Handle<'a, S: Storage+'a>
{
	bs: &'a BufferRing<S>,
	idx: usize,
}
impl<'a, S: 'a + Storage> Handle<'a, S>
{
	pub fn get_index(&self) -> usize {
		self.idx
	}
}
impl<'a, S: 'a + Storage> ops::Drop for Handle<'a, S>
{
	fn drop(&mut self) {
		panic!("Handles to BufferRing shouldn't be dropped");
	}
}
impl<'a, S: 'a + Storage> ops::Deref for Handle<'a, S>
{
	type Target = S::Inner;
	fn deref(&self) -> &S::Inner {
		// SAFE: This handle has unique access to the accessed element
		unsafe {
			&*(*self.bs.data.get()).get(self.idx)
		}
	}
}
impl<'a, S: 'a + Storage> ops::DerefMut  for Handle<'a, S>
{
	fn deref_mut(&mut self) -> &mut S::Inner {
		// SAFE: This handle has unique access to the accessed element
		unsafe {
			&mut *(*self.bs.data.get()).get(self.idx)
		}
	}
}

impl<T> Storage for [T; 4]
{
	type Inner = T;
	fn len() -> usize { 4 }
	unsafe fn get(&self, i: usize) -> *mut T {
		<[T]>::get_unchecked(self, i) as *const T as *mut T
	}
}
