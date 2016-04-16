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
#![feature(unique,nonzero)]	// Unique/NonZero for smart pointers
#![feature(slice_patterns)]	// Slice (array) destructuring patterns, used by multiboot code
#![feature(step_by)]	// Range::step_by
#![feature(linkage)]	// allows using #[linkage="external"]
#![feature(const_fn)]	// Allows defining `const fn`
#![feature(get_type_id,reflect_marker)] // used by process_local's "AnyMap" hackery
#![cfg_attr(not(use_acpica),feature(ptr_as_ref))]	// used by ACPI code (custom impl, not ACPICA)
#![feature(unsafe_no_drop_flag,filling_drop)]	// Used by smart pointers to reduce size
#![feature(unicode)]

#![no_std]

#![deny(not_tagged_safe)]

#![feature(plugin)]
#![feature(custom_attribute)]
#![plugin(tag_safe)]

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
#[macro_use] #[cfg(arch="amd64")] #[path="arch/amd64/mod-macros.rs"] pub mod arch_macros;

// Evil Hack: For some reason, write! (and friends) will expand pointing to std instead of core
#[doc(hidden)]
mod std {
	pub use core::option;
	pub use core::{default,fmt,cmp};
	pub use core::marker;	// needed for derive(Copy)
	pub use core::iter;	// needed for 'for'
}

/// Kernel's version of 'std::prelude'
pub mod prelude;

/// Library datatypes (Vec, Queue, ...)
#[macro_use]
pub mod lib;	// Clone of libstd

mod symbols;

/// Heavy synchronisation primitives (Mutex, Semaphore, RWLock, ...)
#[macro_use]
pub mod sync;

/// Asynchrnous wait support
pub mod async;

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

/// Kernel version (with build number)
pub const VERSION_STRING: &'static str = concat!("Tifflin Kernel v", env!("TK_VERSION"), " build ", env!("TK_BUILD"));
/// Kernel build information (git hash and compiler)
pub const BUILD_STRING: &'static str = concat!("Git state : ", env!("TK_GITSPEC"), ", Built with ", env!("RUST_VERSION"));

// vim: ft=rust

