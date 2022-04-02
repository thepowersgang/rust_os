// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/macros.rs
// - Architecture-provided macros

/// Emits a distinctive instruction (with no effect)
// SAFE: No-op
macro_rules! CHECKMARK{ () => (unsafe { ::core::arch::asm!("xchg cx, cx", options(nostack));}); }

// vim: ft=rust
