// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/sync.rs
// - Lightweight spinlock
use core::kinds::{Send,Sync};

/// Lightweight protecting spinlock
pub struct Spinlock<T: Send>
{
	pub lock: ::core::atomic::AtomicBool,
	pub value: ::core::cell::UnsafeCell<T>,
}

/// Handle to the held spinlock
pub struct HeldSpinlock<'lock,T:'lock+Send>
{
	lock: &'lock mut Spinlock<T>,
	if_set: bool,
}

unsafe impl<T: Send> Sync for Spinlock<T> {}

impl<T: Send> Spinlock<T>
{
	pub fn lock<'_self>(&'_self self) -> HeldSpinlock<'_self,T> {
		unsafe {
			(*(self as *const _ as *mut Spinlock<T>)).lock_impl()
		}
	}
	fn lock_impl<'_self>(&'_self mut self) -> HeldSpinlock<'_self,T>
	{
		let if_set = unsafe {
			let mut flags: uint;
			asm!("pushf\npop $0\ncli" : "=r" (flags));
			while self.lock.compare_and_swap(false, true, ::core::atomic::Ordering::Relaxed) == true
			{
			}
			(flags & 0x200) != 0
			};
		//::arch::puts("Spinlock::lock() - Held\n");
		HeldSpinlock { lock: self, if_set: if_set }
	}
	
	pub fn release(&mut self, set_if: bool)
	{
		//::arch::puts("Spinlock::release()\n");
		self.lock.store(false, ::core::atomic::Ordering::Relaxed);
		if set_if {
			unsafe { asm!("sti" : : : : "volatile"); }
		}
	}
}

#[unsafe_destructor]
impl<'lock,T: Send> ::core::ops::Drop for HeldSpinlock<'lock, T>
{
	fn drop(&mut self)
	{
		self.lock.release(self.if_set);
	}
}

impl<'lock,T: Send> ::core::ops::Deref for HeldSpinlock<'lock, T>
{
	type Target = T;
	fn deref<'a>(&'a self) -> &'a T {
		unsafe { &*self.lock.value.get() }
	}
}
impl<'lock,T: Send> ::core::ops::DerefMut for HeldSpinlock<'lock, T>
{
	fn deref_mut<'a>(&'a mut self) -> &'a mut T {
		unsafe { &mut *self.lock.value.get() }
	}
}

// vim: ft=rust

