// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/main.rs
// - Kernel library root
#![crate_name="kernel"]
#![crate_type="lib"]
#![feature(unsize,coerce_unsized)]	// For DST smart pointers
#![feature(core_intrinsics)]	// Intrinsics
#![feature(box_patterns)]	// Used in boxed::unwrap
#![feature(thread_local)]	// Allows use of thread_local
#![feature(lang_items)]	// Allow definition of lang_items
#![feature(auto_traits)]	// POD trait
#![feature(negative_impls)]	// Negative impls
#![feature(linkage)]	// allows using #[linkage="external"]
//#![feature(integer_atomics)]	// AtomicU8
#![feature(dropck_eyepatch)]
#![feature(panic_info_message)]
#![feature(extern_types)]
#![feature(cfg_target_has_atomic)]	// #[cfg(target_has_atomic="64")]

#![allow(special_module_name)]

//#![cfg_attr(target_arch="riscv64",feature(const_raw_ptr_to_usize_cast))]

#![cfg_attr(not(feature="test"),no_std)]
#![cfg_attr(feature="test",allow(dead_code,unused_imports))]

//#![deny(not_tagged_safe)]
//#![feature(plugin)]
//#![plugin(tag_safe)]

#[cfg(feature="test")]
extern crate core;
#[macro_use]
extern crate alloc;
// HACK: This also exports the module :(
pub use alloc::vec;
pub use alloc::format;

#[allow(unused_imports)]
use prelude::*;

extern crate stack_dst;

//#[repr(C)]	// (not needed)
pub enum Void {}
extern "C" {
	type Extern;
}

pub use arch::memory::PAGE_SIZE;

#[doc(hidden)]
#[macro_use] pub mod logmacros;
#[doc(hidden)]
#[macro_use] pub mod macros;
#[doc(hidden)]

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
pub mod user_async;
//#[path="async-v3/mod.rs"]
//pub mod _async3;
pub mod futures;

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

/// Kernel configuration
pub mod config;

/// Stack unwinding (panic) handling
#[cfg(not(any(feature="test",test)))]
pub mod unwind;

pub mod irqs;

/// Built-in device drivers
pub mod hw;

/// Achitecture-specific code
pub mod arch;

pub mod build_info {
	#[repr(C)]
	struct Str {
		len: u16,
		bytes: [u8; 0],
	}
	impl Str {
		// UNSAFE: Caller must ensure that the source is trusted
		unsafe fn get_str(&self) -> &str {
			let len = self.len as usize;
			let ptr = self.bytes.as_ptr();
			core::str::from_utf8_unchecked( core::slice::from_raw_parts(ptr, len) )
		}
	}
	extern "C" {
		static BUILD_STRING: Str;
		static VERSION_STRING: Str;
	}

	pub fn build_string() -> &'static str {
		// SAFE: Valid string
		unsafe {
			BUILD_STRING.get_str()
		}
	}
	pub fn version_string() -> &'static str {
		// SAFE: Valid string
		unsafe {
			VERSION_STRING.get_str()
		}
	}

	// HACK: This should only be set when building for RLS/analyser
	#[cfg(any(feature="test", windows))]
	pub mod _test {
		use super::Str;
		#[no_mangle]
		static BUILD_STRING: Str = Str { len: 0, bytes: [] };
		#[no_mangle]
		static VERSION_STRING: Str = Str { len: 0, bytes: [] };
	}
}

// vim: ft=rust

