// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/mod.rs
//! Common blocking synchronisation primitives
pub use arch::sync::Spinlock;

pub use sync::mutex::Mutex;

#[macro_use]
pub mod mutex;

// vim: ft=rust

