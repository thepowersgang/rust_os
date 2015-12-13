// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/mod.rs
// - Blocking synchronisation primitives
pub use arch::sync::Spinlock;
pub use arch::sync::hold_interrupts;

pub use sync::mutex::Mutex;
pub use sync::semaphore::Semaphore;
pub use sync::rwlock::RwLock;
pub use sync::event_channel::{EventChannel,EVENTCHANNEL_INIT};
pub use self::atomic::AtomicU32;

#[macro_use]
pub mod mutex;

pub mod semaphore;

pub mod rwlock;

pub mod event_channel;

pub mod atomic;

// vim: ft=rust

