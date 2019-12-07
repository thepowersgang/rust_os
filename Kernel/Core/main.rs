// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/main.rs
// - Kernel library root
#![crate_name="kernel"]
#![crate_type="lib"]
#![feature(unsize,coerce_unsized)]	// For DST smart pointers
#![feature(core_intrinsics)]	// Intrinsics
#![feature(asm)]	// Enables the asm! syntax extension
#![feature(box_syntax)]	// Enables 'box' syntax
#![feature(box_patterns)]	// Used in boxed::unwrap
#![feature(thread_local)]	// Allows use of thread_local
#![feature(lang_items)]	// Allow definition of lang_items
#![feature(optin_builtin_traits)]	// Negative impls
#![feature(slice_patterns)]	// Slice (array) destructuring patterns, used by multiboot code
#![feature(linkage)]	// allows using #[linkage="external"]
#![feature(const_fn)]	// Allows defining `const fn`
//#![feature(integer_atomics)]	// AtomicU8
#![feature(dropck_eyepatch)]
#![feature(panic_info_message)]

#![cfg_attr(not(feature="test"),no_std)]

//#![deny(not_tagged_safe)]
//#![feature(plugin)]
//#![plugin(tag_safe)]

#[cfg(feature="test")]
extern crate core;

#[allow(unused_imports)]
use prelude::*;

extern crate stack_dst;

//#[repr(C)]	// (not needed)
pub enum Void {}

pub use arch::memory::PAGE_SIZE;

#[doc(hidden)]
#[macro_use] pub mod logmacros;
#[doc(hidden)]
#[macro_use] pub mod macros;
#[doc(hidden)]
#[macro_use] #[cfg(any(arch="amd64", target_arch="x86_64"))] #[path="arch/amd64/mod-macros.rs"] pub mod arch_macros;

/// Kernel's version of 'std::prelude'
pub mod prelude;

/// Library datatypes (Vec, Queue, ...)
#[macro_use]
pub mod lib;	// Clone of libstd

#[cfg(not(feature="test"))]
mod symbols;

/// Heavy synchronisation primitives (Mutex, Semaphore, RWLock, ...)
#[macro_use]
pub mod sync;

/// Asynchrnous wait support
pub mod async;
#[path="async-v3/mod.rs"]
pub mod _async3;

/// Logging framework
pub mod logging;
/// Memory management (physical, virtual, heap)
pub mod memory;
/// Thread management
#[macro_use]
pub mod threads;
/// Timekeeping (timers and wall time)
pub mod time;

/// Module management (loading and initialisation of kernel modules)
pub mod modules;

/// Meta devices (the Hardware Abstraction Layer)
pub mod metadevs;
/// Device to driver mapping manager
///
/// Starts driver instances for the devices it sees
pub mod device_manager;

// Public for driver modules
pub mod vfs;

/// Kernel configuration
pub mod config;

/// Stack unwinding (panic) handling
pub mod unwind;

pub mod irqs;

/// Built-in device drivers
mod hw;

/// Achitecture-specific code
pub mod arch;

cfg_if::cfg_if!{
    if #[cfg(feature="test")] {
    }
    else {
        /// Kernel version (with build number)
        pub const VERSION_STRING: &'static str = concat!("Tifflin Kernel v", env!("TK_VERSION"), " build ", env!("TK_BUILD"));
        /// Kernel build information (git hash and compiler)
        pub const BUILD_STRING: &'static str = concat!("Git state : ", env!("TK_GITSPEC"), ", Built with ", env!("RUST_VERSION"));
    }
}

// vim: ft=rust

