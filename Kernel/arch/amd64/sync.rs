// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/sync.rs
// - Lightweight spinlock

/// Lightweight protecting spinlock
pub struct Spinlock<T>
{
	pub lock: uint,
	pub value: T,
}

/// Handle to the held spinlock
pub struct HeldSpinlock<'lock,T:'lock>
{
	lock: &'lock mut Spinlock<T>,
	if_set: bool,
}

impl<T> Spinlock<T>
{
	pub fn lock<'_self>(&'_self mut self) -> HeldSpinlock<'_self,T>
	{
		let if_set = unsafe {
			let mut flags: uint;
			asm!("
				pushf
				pop $0
				cli
				1:
				xchg $1, ($2)
				test $1, $1
				jnz 1
				"
				: "=r" (flags)
				: "r" (1u), "r"(&self.lock)
				: "$0"
				: "volatile"
				);
			flags & 0x200 != 0
			};
		//::arch::puts("Spinlock::lock() - Held\n");
		HeldSpinlock { lock: self, if_set: if_set }
	}
	
	pub fn release(&mut self, set_if: bool)
	{
		//::arch::puts("Spinlock::release()\n");
		self.lock = 0;
		if set_if {
			unsafe { asm!("sti" : : : : "volatile"); }
		}
	}
}

#[unsafe_destructor]
impl<'lock,T> ::core::ops::Drop for HeldSpinlock<'lock, T>
{
	fn drop(&mut self)
	{
		self.lock.release(self.if_set);
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

// vim: ft=rust

