// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/macros.rs
// - Architecture-provided macros
#![macro_escape]	// Let macros be accessible by parent

#[macro_export]
macro_rules! spinlock_init( ($val:expr) => ( ::arch::sync::Spinlock { lock: 0, value: $val}) )

// vim: ft=rust
