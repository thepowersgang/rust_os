//
//
//
#![macro_escape]

pub struct Mutex<T>
{
	pub locked_held: ::sync::Spinlock<bool>,
	pub queue: ::core::cell::UnsafeCell<::threads::WaitQueue>,
	pub val: ::core::cell::UnsafeCell<T>,
}

struct HeldMutex<'lock,T:'lock>
{
	lock: &'lock Mutex<T>
}

impl<T> Mutex<T>
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

#[unsafe_destructor]
impl<'lock,T> ::core::ops::Drop for HeldMutex<'lock,T>
{
	fn drop(&mut self) {
		self.lock.unlock();
	}
}
impl<'lock,T> ::core::ops::Deref<T> for HeldMutex<'lock,T>
{
	fn deref<'a>(&'a self) -> &'a T {
		unsafe { &*self.lock.val.get() }
	}
}
impl<'lock,T> ::core::ops::DerefMut<T> for HeldMutex<'lock,T>
{
	fn deref_mut<'a>(&'a mut self) -> &'a mut T {
		unsafe { &mut *self.lock.val.get() }
	}
}

#[macro_export]
macro_rules! mutex_init( ($val:expr) => (::sync::mutex::Mutex{
	locked_held: spinlock_init!(false),
	queue: ::core::cell::UnsafeCell { value: ::threads::WAITQUEUE_INIT },
	val: ::core::cell::UnsafeCell{ value: $val },
	}) )

// vim: ft=rust

