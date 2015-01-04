//
//
//
#![macro_escape]
use lib::LazyStatic;
use core::kinds::{Send, Sync};

pub struct Mutex<T: Send>
{
	pub locked_held: ::sync::Spinlock<bool>,
	pub queue: ::core::cell::UnsafeCell<::threads::WaitQueue>,
	pub val: ::core::cell::UnsafeCell<T>,
}

struct HeldMutex<'lock,T:'lock+Send>
{
	lock: &'lock Mutex<T>
}

pub struct LazyMutex<T: Send>(pub Mutex<LazyStatic<T>>);

unsafe impl<T> Sync for Mutex<T>
{
}

impl<T: Send> Mutex<T>
{
	/*
	pub fn new(val: T) -> Mutex<T> {
		Mutex {
			locked_held: spinlock_init!(false),
			queue: ::threads::WAITQUEUE_INIT,
			val: val,
		}
	}
	*/
	
	pub fn lock(&self) -> HeldMutex<T> {
		{
			let mut held = self.locked_held.lock();
			if *held != false
			{
				unsafe { (*self.queue.get()).wait(held) };
			}
			else
			{
				*held = true;
			}
		}
		return HeldMutex { lock: self };
	}
	pub fn unlock(&self) {
		let mut held = self.locked_held.lock();
		*held = false;
		unsafe { (*self.queue.get()).wake_one() };
		// TODO: Wake anything waiting
	}
}

impl<T: Send> LazyMutex<T>
{
	pub fn lock(&self, init_fcn: | | -> T) -> HeldMutex<LazyStatic<T>>
	{
		let mut lh = self.0.lock();
		lh.prep(init_fcn);
		lh
	}
}

#[unsafe_destructor]
impl<'lock,T:Send> ::core::ops::Drop for HeldMutex<'lock,T>
{
	fn drop(&mut self) {
		self.lock.unlock();
	}
}
impl<'lock,T:Send> ::core::ops::Deref<T> for HeldMutex<'lock,T>
{
	fn deref<'a>(&'a self) -> &'a T {
		unsafe { &*self.lock.val.get() }
	}
}
impl<'lock,T:Send> ::core::ops::DerefMut<T> for HeldMutex<'lock,T>
{
	fn deref_mut<'a>(&'a mut self) -> &'a mut T {
		unsafe { &mut *self.lock.val.get() }
	}
}

#[macro_export]
macro_rules! mutex_init{ ($val:expr) => (::sync::mutex::Mutex{
	locked_held: spinlock_init!(false),
	queue: ::core::cell::UnsafeCell { value: ::threads::WAITQUEUE_INIT },
	val: ::core::cell::UnsafeCell{ value: $val },
	}) }
macro_rules! lazymutex_init{
	() => {::sync::mutex::LazyMutex(mutex_init!( ::lib::LazyStatic(None) ))}
}

// vim: ft=rust

