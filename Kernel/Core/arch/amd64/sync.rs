// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/sync.rs
//! Low-level synchronisaion primitives
use core::sync::atomic::{AtomicBool,Ordering};

const TRACE_IF: bool = false;
//const TRACE_IF: bool = true;

/// Lightweight protecting spinlock
pub struct Spinlock<T>
{
	#[doc(hidden)]
	pub lock: AtomicBool,
	#[doc(hidden)]
	pub value: ::core::cell::UnsafeCell<T>,
}
unsafe impl<T: Send> Sync for Spinlock<T> {}

/// Handle to the held spinlock
pub struct HeldSpinlock<'lock,T:'lock>
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

impl<T> Spinlock<T>
{
	/// Create a new spinning lock
	pub const fn new(val: T) -> Spinlock<T> {
		Spinlock {
			lock: AtomicBool::new(false),
			value: ::core::cell::UnsafeCell::new(val),
		}
	}
	pub fn get_mut(&mut self) -> &mut T {
		// SAFE: &mut to lock
		unsafe { &mut *self.value.get() }
	}
	
	/// Lock this spinning lock
	//#[not_safe(irq)]
	pub fn lock(&self) -> HeldSpinlock<T>
	{
		self.inner_lock();
		HeldSpinlock { lock: self }
	}

	/// Lock this spinning lock (accepting risk of panick/deadlock from IRQs)
	//#[is_safe(irq)]
	pub fn lock_irqsafe(&self) -> HeldSpinlock<T> {
		self.inner_lock();
		HeldSpinlock { lock: self }
	}
	/// Attempt to acquire the lock, returning None if it is already held by this CPU
	//#[is_safe(irq)]
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
	
	fn inner_lock(&self) {
		//while self.lock.compare_and_swap(0, cpu_num()+1, Ordering::Acquire) != 0
		while self.lock.compare_and_swap(false, true, Ordering::Acquire) == true
		{
		}
		::core::sync::atomic::fence(Ordering::Acquire);
	}
	fn inner_release(&self) {
		//::arch::puts("Spinlock::release()\n");
		::core::sync::atomic::fence(Ordering::Release);
		self.lock.store(false, Ordering::Release);
	}
}
// Some special functions on non-wrapping spinlocks
impl Spinlock<()>
{
	pub unsafe fn unguarded_lock(&self) {
		self.inner_lock()
	}
	pub unsafe fn unguarded_release(&self) {
		self.inner_release()
	}
}
impl<T: Default> Default for Spinlock<T>
{
	fn default() -> Self {
		Spinlock::new(Default::default())
	}
}

impl<'lock,T> ::core::ops::Drop for HeldSpinlock<'lock, T>
{
	fn drop(&mut self)
	{
		self.lock.inner_release();
	}
}

impl<'lock,T> ::core::ops::Deref for HeldSpinlock<'lock, T>
{
	type Target = T;
	fn deref(&self) -> &T {
		// SAFE: & to handle makes & to value valid
		unsafe { &*self.lock.value.get() }
	}
}
impl<'lock,T> ::core::ops::DerefMut for HeldSpinlock<'lock, T>
{
	fn deref_mut(&mut self) -> &mut T {
		// SAFE: &mut to handle makes &mut to value valid
		unsafe { &mut *self.lock.value.get() }
	}
}

/// Prevent interrupts from firing until return value is dropped
pub fn hold_interrupts() -> HeldInterrupts
{
	// SAFE: Correct inline assembly
	let if_set = unsafe {
		let flags: u64;
		asm!("pushf; pop $0; cli" : "=r" (flags) : : "memory" : "volatile");
		(flags & 0x200) != 0
		};
	
	if TRACE_IF {
		if if_set {
			::arch::puts("hold_interrupts() - IF cleared\n");
		}
		else {
			::arch::puts("hold_interrupts() - IF maintained\n");
		}
	}
	HeldInterrupts(if_set)
}

impl ::core::ops::Drop for HeldInterrupts
{
	fn drop(&mut self)
	{
		if TRACE_IF {
			if self.0 {
				::arch::puts("HeldInterrupts::drop() - IF set\n");
			}
			else {
				::arch::puts("HeldInterrupts::drop() - IF maintained\n");
			}
		}
		
		if self.0 {
			// SAFE: Just re-enables interrupts
			unsafe { asm!("sti" : : : "memory" : "volatile"); }
		}
	}
}

pub unsafe fn stop_interrupts() {
	asm!("cli" : : : : "volatile");
}
pub unsafe fn start_interrupts() {
	asm!("sti" : : : : "volatile");
}

// vim: ft=rust

