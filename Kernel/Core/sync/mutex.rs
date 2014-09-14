//
//
//
#![macro_escape]

pub struct Mutex<T>
{
	pub locked_held: ::sync::Spinlock<bool>,
	pub queue: ::threads::WaitQueue,
	pub val: T,
}

struct HeldMutex<'lock,T:'lock>
{
	lock: &'lock mut Mutex<T>
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
	
	pub fn lock(&mut self) -> HeldMutex<T> {
		{
			let mut held = self.locked_held.lock();
			if *held != false
			{
				fail!("TODO: Mutex.lock wait");
			}
			*held = true;
		}
		return HeldMutex { lock: self };
	}
	pub fn unlock(&mut self) {
		let mut held = self.locked_held.lock();
		*held = false;
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
		&self.lock.val
	}
}
impl<'lock,T> ::core::ops::DerefMut<T> for HeldMutex<'lock,T>
{
	fn deref_mut<'a>(&'a mut self) -> &'a mut T {
		&mut self.lock.val
	}
}

#[macro_export]
macro_rules! mutex_init( ($val:expr) => (::sync::mutex::Mutex{
	locked_held: spinlock_init!(false),
	queue: ::threads::WAITQUEUE_INIT,
	val: $val,
	}) )

// vim: ft=rust

