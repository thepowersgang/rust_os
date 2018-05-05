// Tifflin OS - Standard Library (clone)
// - By John Hodge (thePowersGang)
//
// A clone of rust's libstd customised to work correctly on Tifflin
#![crate_type="rlib"]
#![crate_name="std"]
#![feature(lang_items)]	// Allow definition of lang_items
#![feature(linkage)]	// Used for low-level runtime
#![feature(core_intrinsics)]
#![feature(core_char_ext,core_str_ext,core_slice_ext)]
#![feature(const_fn)]
#![feature(box_syntax)]
#![feature(raw)]
#![feature(slice_concat_ext)]
#![feature(use_extern_macros)]	// allow `use` with macros
#![feature(alloc,allocator_api)]
#![feature(allocator_internals)]
#![feature(core_panic_info)]	// Needed because of import of `panic` macro bringin in the module too
#![default_lib_allocator]
#![no_std]

#[macro_use]
extern crate syscalls;
#[macro_use]
extern crate macros;

extern crate alloc;
extern crate alloc_system;

//extern crate loader;
// Macros
pub use alloc::{/*vec, */format};
pub use core::{try, assert_eq, panic, write, unreachable, unimplemented};


// Raw re-exports from core
pub use core::{option, result};
pub use core::{/*slice, */str, ptr, char};
pub use core::{iter, clone};
pub use core::{mem, cmp, ops};
pub use core::{default, cell};
pub use core::convert;
pub use core::intrinsics;
pub use core::marker;
pub use core::num;
pub use core::raw;

// Crate re-exports
pub use alloc::{rc,boxed};
pub use alloc::slice;

mod std {
	pub use core::{option, result};
	pub use fmt;
	pub use core::iter;
	pub use core::{mem, cmp, ops};
	pub use core::{str};
	pub use core::convert;
	pub use ffi;
}

/// Prelude
pub mod prelude {
	pub mod v1 {
		pub use core::marker::{Copy,Send,Sync,Sized};
		pub use core::ops::{Drop,Fn,FnMut,FnOnce};
		pub use core::mem::drop;
		pub use alloc::boxed::Box;
		pub use borrow::ToOwned;
		pub use core::clone::Clone;
		pub use core::cmp::{PartialEq, PartialOrd, Eq, Ord};
		pub use core::convert::{AsRef,AsMut,Into,From};
		pub use core::default::Default;
		pub use core::iter::{Iterator,Extend,IntoIterator};
		pub use core::iter::{DoubleEndedIterator, ExactSizeIterator};
		
		pub use core::option::Option::{self,Some,None};
		pub use core::result::Result::{self,Ok,Err};

		pub use slice::SliceConcatExt;

		pub use string::{String,ToString};
		pub use alloc::Vec;
	}
}

/// Formatted output
pub mod fmt {
	pub use core::fmt::*;

	pub fn format(args: ::std::fmt::Arguments) -> ::string::String
	{
		let mut output = ::string::String::new();
		let _ = output.write_fmt(args);
		output
	}

}

pub mod collections {
	//pub use alloc::BTreeMap;
}

mod start;

pub mod ffi;

pub mod hash;

pub mod env;

pub extern crate std_io as io;
pub extern crate std_rt as rt;
pub extern crate std_sync as sync;

pub mod fs;

pub mod error;

pub use alloc::{vec, string, borrow};

pub mod os;

pub mod heap;

