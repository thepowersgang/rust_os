// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/mod.rs
//! Common blocking synchronisation primitives
pub use arch::sync::Spinlock;

pub use sync::mutex::Mutex;
pub use sync::event_channel::{EventChannel,EVENTCHANNEL_INIT};

#[macro_use]
pub mod mutex;

pub mod rwlock;

pub mod event_channel;

// vim: ft=rust

