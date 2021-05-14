// Tifflin OS - Usermod Synchronisation
// - By John Hodge (thePowersGang)
//
//! Usermode synchronisation primitives
#![no_std]

extern crate syscalls;

pub use mutex::Mutex;
pub use rwlock::RwLock;

pub mod mutex;
pub mod rwlock;

pub use core::sync::atomic;


