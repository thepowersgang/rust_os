// Tifflin OS - Usermod Synchronisation
// - By John Hodge (thePowersGang)
//
//! Usermode synchronisation primitives
#![feature(const_fn)]
#![no_std]

pub use mutex::Mutex;

pub mod mutex;

pub use core::sync::atomic;


