//
//
//
#![macro_escape]	// Let macros be accessible by parent


pub struct Spinlock<T>
{
	pub lock: uint,
	pub value: T,
}

pub struct HeldSpinlock<'lock,T:'lock>
{
	lock: &'lock mut Spinlock<T>,
}

impl<T> Spinlock<T>
{
	pub fn lock<'_self>(&'_self mut self) -> HeldSpinlock<'_self,T>
	{
		unsafe {
			let mut v = 0u;
			asm!("
				cli
				1:
				xchg $0, ($1)
				test $0, $0
				jnz 1
				"
				: /* no outputs */
				: "r" (1u), "r"(&self.lock)
				: "$0"
				: "volatile"
				);
		}
		//::arch::puts("Spinlock::lock() - Held\n");
		HeldSpinlock { lock: self }
	}
	
	pub fn release(&mut self)
	{
		//::arch::puts("Spinlock::release()\n");
		self.lock = 0;
		//unsafe { asm!("sti" : : : : "volatile"); }
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

