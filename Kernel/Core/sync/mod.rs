// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/mod.rs
// - Blocking synchronisation primitives
pub use crate::arch::sync::Spinlock;
pub use crate::arch::sync::hold_interrupts;

pub use crate::sync::mutex::Mutex;
pub use crate::sync::semaphore::Semaphore;
pub use crate::sync::rwlock::RwLock;
pub use crate::sync::event_channel::EventChannel;
pub use self::queue::Queue;

#[macro_use]
pub mod mutex;

pub mod semaphore;

pub mod rwlock;

pub mod event_channel;
pub mod queue;

// vim: ft=rust

