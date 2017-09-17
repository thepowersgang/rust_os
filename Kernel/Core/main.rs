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
#![feature(unique,shared,nonzero)]	// Unique/Shared/NonZero for smart pointers
#![feature(slice_patterns)]	// Slice (array) destructuring patterns, used by multiboot code
#![feature(iterator_step_by)]	// Iterator::step_by
#![feature(linkage)]	// allows using #[linkage="external"]
#![feature(const_fn)]	// Allows defining `const fn`
#![feature(get_type_id)] // used by process_local's "AnyMap" hackery
#![feature(drop_types_in_const)]	// Allows `statics` to contain destructors
#![feature(placement_new_protocol)]	// Used for Box<T> (mrustc support)
#![feature(integer_atomics)]	// AtomicU8
#![feature(generic_param_attrs,dropck_eyepatch)]
#![feature(const_atomic_bool_new,const_atomic_ptr_new,const_atomic_usize_new,const_unsafe_cell_new,const_unique_new)]	// Various const fns

#![no_std]

#![deny(not_tagged_safe)]

#![feature(plugin)]
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

/// Kernel version (with build number)
pub const VERSION_STRING: &'static str = concat!("Tifflin Kernel v", env!("TK_VERSION"), " build ", env!("TK_BUILD"));
/// Kernel build information (git hash and compiler)
pub const BUILD_STRING: &'static str = concat!("Git state : ", env!("TK_GITSPEC"), ", Built with ", env!("RUST_VERSION"));

// vim: ft=rust

