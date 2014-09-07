// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/crate.rs
// - AMD64/x86_64 architecture support
#![feature(asm)]
#![crate_type="lib"]
#![no_std]
extern crate core;
extern crate common;

pub use log::{puts, puth};

pub mod float;
pub mod interrupts;
pub mod memory;
pub mod boot;

mod log;
mod x86_io;

// vim: ft=rust

