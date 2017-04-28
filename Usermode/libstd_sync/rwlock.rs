// Tifflin OS - Usermode Synchronisation
// - By John Hodge (thePowersGang)
//
//! Reader-writer lock
use core::ops;
use core::cell::UnsafeCell;
use mutex::Mutex;

pub struct RwLock<T: ?Sized>
{
	int: ::mutex::Mutex<Inner>,
	data: UnsafeCell<T>,
}
unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}
unsafe impl<T: ?Sized + Send> Sync for RwLock<T> {}

struct Inner
{
	readers: usize,
	writers: usize,
}

impl<T> RwLock<T>
{
	pub const fn new(v: T) -> RwLock<T> {
		RwLock {
			int: Mutex::new(Inner {
				readers: 0,
				writers: 0,
				}),
			data: UnsafeCell::new(v),
			}
	}
}

impl<T: ?Sized> RwLock<T>
{
	pub fn write(&self) -> Write<T> {
		loop {
			let mut lh = self.int.lock();
			if lh.readers > 0 {
				panic!("TODO: RwLock::write - wait for readers to release");
			}
			else {
				lh.writers += 1;
				if lh.writers > 1 {
					panic!("TODO: RwLock::write - wait for other writer to release");
				}
				return Write { p: self };
			}
		}
	}
	pub fn read(&self) -> Read<T> {
		loop {
			let mut lh = self.int.lock();
			if lh.readers > 0 {
				lh.readers += 1;
				return Read { p: self };
			}
			else if lh.writers > 0 {
				panic!("TODO: RwLock::read - wait for writer to release");
			}
			else {
				lh.readers += 1;
				return Read { p: self };
			}
		}
	}

	pub fn get_mut(&mut self) -> &mut T {
		// SAFE: mut handle to UnsafeCell
		unsafe { &mut *self.data.get() }
	}
}

pub struct Read<'a, T: ?Sized + 'a> {
	p: &'a RwLock<T>,
}
impl<'a, T: 'a + ?Sized> ops::Deref for Read<'a, T> {
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: Read handle can read
		unsafe { &*self.p.data.get() }
	}
}
impl<'a, T: 'a + ?Sized> ops::Drop for Read<'a, T> {
	fn drop(&mut self) {
		let mut lh = self.p.int.lock();
		lh.readers -= 1;
		if lh.readers == 0 && lh.writers > 0 {
			panic!("TODO: rwlock::Read::drop - Wake writers");
		}
	}
}

pub struct Write<'a, T: ?Sized + 'a> {
	p: &'a RwLock<T>,
}
impl<'a, T: 'a + ?Sized> ops::Deref for Write<'a, T> {
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: Write handle can do anything
		unsafe { &*self.p.data.get() }
	}
}
impl<'a, T: 'a + ?Sized> ops::DerefMut for Write<'a, T> {
	fn deref_mut(&mut self) -> &mut T {
		// SAFE: Write handle can do anything
		unsafe { &mut *self.p.data.get() }
	}
}
impl<'a, T: 'a + ?Sized> ops::Drop for Write<'a, T> {
	fn drop(&mut self) {
		let mut lh = self.p.int.lock();
		lh.writers -= 1;
		if lh.writers > 0 {
			panic!("TODO: rwlock::Write::drop - Wake writers");
		}
		else if lh.readers > 0 {
			panic!("TODO: rwlock::Write::drop - Wake readers");
		}
		else {
			// Uncontended release
		}
	}
}


