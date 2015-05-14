// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/macros.rs
// - Architecture-provided macros

/// Emits a distinctive instruction (with no effect)
macro_rules! CHECKMARK{ () => (unsafe { asm!("xchg %cx, %cx" : : : : "volatile");}); }

#[macro_export]
macro_rules! spinlock_init {
	($val:expr) => ( ::arch::sync::Spinlock { lock: ::core::atomic::ATOMIC_BOOL_INIT, value: ::core::cell::UnsafeCell { value: $val } })
	}

// vim: ft=rust
