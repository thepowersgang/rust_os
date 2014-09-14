//
//
//
#![macro_escape]	// Let macros be accessible by parent


pub struct Spinlock<T>
{
	pub lock: uint,
	pub value: T,
}

struct HeldSpinlock<'lock,T:'lock>
{
	lock: &'lock mut Spinlock<T>,
}

impl<T> Spinlock<T>
{
	pub fn lock<'_self>(&'_self mut self) -> HeldSpinlock<'_self,T>
	{
		::arch::puts("Spinlock::lock()\n");
		unsafe {
			asm!(
				"cli"
				"1:"
				"xchgl %0, (%1)"
				"test %0, %0"
				"jnz 1"
				: 
				: "r" (1u), "r"(&self.lock)
				);
		}
		::arch::puts("Spinlock::lock() - Held\n");
		HeldSpinlock { lock: self }
	}
	
	pub fn release(&mut self)
	{
		::arch::puts("Spinlock::release()\n");
		unsafe {
			self.lock = 0;
			//asm!("sti");
		}
	}
}

#[unsafe_destructor]
impl<'lock,T> ::core::ops::Drop for HeldSpinlock<'lock, T>
{
	fn drop(&mut self)
	{
		self.lock.release();
	}
}

impl<'lock,T> ::core::ops::Deref<T> for HeldSpinlock<'lock, T>
{
	fn deref<'a>(&'a self) -> &'a T {
		&self.lock.value
	}
}
impl<'lock,T> ::core::ops::DerefMut<T> for HeldSpinlock<'lock, T>
{
	fn deref_mut<'a>(&'a mut self) -> &'a mut T {
		&mut self.lock.value
	}
}

#[macro_export]
macro_rules! spinlock_init( ($val:expr) => ( ::arch::sync::Spinlock { lock: 0, value: $val}) )

// vim: ft=rust

