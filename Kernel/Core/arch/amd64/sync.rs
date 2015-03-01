// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/sync.rs
// - Lightweight spinlock
use core::prelude::*;
use core::atomic::{AtomicBool,ATOMIC_BOOL_INIT,Ordering};

/// Lightweight protecting spinlock
pub struct Spinlock<T: Send>
{
	pub lock: ::core::atomic::AtomicBool,
	pub value: ::core::cell::UnsafeCell<T>,
}
unsafe impl<T: Send> Sync for Spinlock<T> {}

/// Handle to the held spinlock
pub struct HeldSpinlock<'lock,T:'lock+Send>
{
	lock: &'lock Spinlock<T>,
}

/// A handle for frozen interrupts
pub struct HeldInterrupts(bool);

///// Handle for a held spinlock that holds interrupts too
//pub struct HeldSpinlockInt<'lock,T:'lock+Send>
//{
//	lock: &'lock Spinlock<T>,
//	irqs: HeldInterrupts,
//}

impl<T: Send> Spinlock<T>
{
	pub fn new(val: T) -> Spinlock<T> {
		Spinlock {
			lock: AtomicBool::new(false),
			value: ::core::cell::UnsafeCell::new(val),
		}
	}
	
	pub fn lock(&self) -> HeldSpinlock<T>
	{
		//while self.lock.compare_and_swap(0, cpu_num()+1, Ordering::Acquire) != 0
		while self.lock.compare_and_swap(false, true, Ordering::Acquire) == true
		{
		}
		::core::atomic::fence(Ordering::Acquire);
		HeldSpinlock { lock: self }
	}
	pub fn try_lock_cpu(&self) -> Option<HeldSpinlock<T>>
	{
		//if self.lock.compare_and_swap(0, cpu_num()+1, Ordering::Acquire) == 0
		if self.lock.compare_and_swap(false, true, Ordering::Acquire) == false
		{
			Some( HeldSpinlock { lock: self } )
		}
		else
		{
			None
		}
	}
	
	fn release(&self)
	{
		//::arch::puts("Spinlock::release()\n");
		::core::atomic::fence(Ordering::Release);
		self.lock.store(false, Ordering::Release);
	}
}

#[unsafe_destructor]
impl<'lock,T: Send> ::core::ops::Drop for HeldSpinlock<'lock, T>
{
	fn drop(&mut self)
	{
		self.lock.release();
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

/// Prevent interrupts from firing until return value is dropped
pub fn hold_interrupts() -> HeldInterrupts
{
	let if_set = unsafe {
		let mut flags: u64;
		asm!("pushf; pop $0; cli" : "=r" (flags) : : "memory" : "volatile");
		(flags & 0x200) != 0
		};
	
	//if ! if_set {
	//	::arch::puts("hold_interrupts() - if_set = false\n");
	//}
	HeldInterrupts(if_set)
}

impl ::core::ops::Drop for HeldInterrupts
{
	fn drop(&mut self)
	{
		if self.0 {
			unsafe {
				asm!("sti" : : : "memory" : "volatile");
			}
		}
	}
}

// vim: ft=rust

