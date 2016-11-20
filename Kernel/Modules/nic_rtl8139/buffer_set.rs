// "Tifflin" Kernel - Networking Stack
// - By John Hodge (thePowersGang)
//
//! Container for a set of owned objects from a pool
use core::ops;

pub type BufferSet4<V> = BufferSet<[V; 4]>;

pub struct BufferSet<S: Storage>
{
	inner: ::kernel::sync::Spinlock<Inner>,
	data: ::core::cell::UnsafeCell<S>,
}
unsafe impl<S: Storage + Send> Send for BufferSet<S> {}
unsafe impl<S: Storage + Send> Sync for BufferSet<S> {}

#[derive(Default)]
struct Inner
{
	wait_queue: ::kernel::threads::WaitQueue,
	usage_mask: u32,
}

pub trait Storage
{
	type Inner;
	fn len() -> usize;
	unsafe fn get(&self, usize)->*mut Self::Inner;
}

impl<S: Storage> BufferSet<S>
{
	pub fn new(data: S) -> BufferSet<S> {
		BufferSet {
			inner: Default::default(),
			data: ::core::cell::UnsafeCell::new(data),
			}
	}

	fn max_mask() -> u32 {
		(1 << S::len()) - 1
	}
	
	pub fn acquire_wait(&self) -> Handle<S> {
		let mut lh = self.inner.lock();
		while lh.usage_mask == Self::max_mask() {
			waitqueue_wait_ext!(lh, .wait_queue);
			lh = self.inner.lock();
		}
		
		let idx = (0 .. S::len()).position(|i| lh.usage_mask & 1 << i == 0).expect("acquire_wait - Usage not full, but couldn't find a bit");
		lh.usage_mask |= 1 << idx;
		
		Handle {
			bs: self,
			idx: idx,
			}
	}

	/// Release an object by index
	pub unsafe fn release(&self, index: usize) {
		let mut lh = self.inner.lock();

		lh.usage_mask &= !(1 << index);
		if lh.wait_queue.has_waiter() {
			lh.wait_queue.wake_one();
		}
	}
}

pub struct Handle<'a, S: Storage+'a>
{
	bs: &'a BufferSet<S>,
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
		let mut lh = self.bs.inner.lock();

		lh.usage_mask &= !(1 << self.idx);
		if lh.wait_queue.has_waiter() {
			lh.wait_queue.wake_one();
		}
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
