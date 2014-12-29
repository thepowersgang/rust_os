// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/macros.rs
// - Architecture-provided macros
#![macro_escape]	// Let macros be accessible by parent

#[macro_export]
macro_rules! spinlock_init {
	($val:expr) => ( ::arch::sync::Spinlock { lock: ::core::atomic::INIT_ATOMIC_BOOL, value: ::core::cell::UnsafeCell { value: $val } })
	}

// vim: ft=rust
