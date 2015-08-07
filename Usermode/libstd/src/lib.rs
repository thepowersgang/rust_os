// Tifflin OS - Standard Library (clone)
// - By John Hodge (thePowersGang)
//
// A clone of rust's libstd customised to work correctly on Tifflin
#![crate_type="rlib"]
#![crate_name="std"]
#![feature(no_std,core)]
#![feature(lang_items)]	// Allow definition of lang_items
#![feature(linkage)]	// Used for low-level runtime
#![feature(core_intrinsics)]
#![feature(core_char_ext,core_str_ext,core_slice_ext)]
#![feature(const_fn)]
#![feature(unique)]
#![feature(result_expect)]  // goddamnit rustc, I wrote that, I get to use it
//#![staged_api]
//#![feature(staged_api)]
#![no_std]

#[macro_use]
extern crate syscalls;
#[macro_use]
extern crate macros;

//extern crate loader;

// Raw re-exports from core
pub use core::{option, result};
pub use core::{slice, str, ptr};
pub use core::{iter, clone};
pub use core::{mem, cmp, ops};
pub use core::{default, cell};
pub use core::convert;
pub use core::intrinsics;
pub use core::marker;

mod std {
	pub use core::{option, result};
	pub use fmt;
	pub use core::iter;
	pub use core::{mem, cmp, ops};
}

/// Prelude
pub mod prelude {
	pub mod v1 {
		pub use core::marker::{Copy,Send,Sync,Sized};
		pub use core::ops::{Drop,Fn,FnMut,FnOnce};
		pub use core::mem::drop;
		//pub use heap::boxed::Box;
		//pub use core::borrow::ToOwned;
		pub use core::clone::Clone;
		pub use core::cmp::{PartialEq, PartialOrd, Eq, Ord};
		pub use core::convert::{AsRef,AsMut,Into,From};
		pub use core::default::Default;
		pub use core::iter::{Iterator,Extend,IntoIterator};
		pub use core::iter::{DoubleEndedIterator, ExactSizeIterator};
		
		pub use core::option::Option::{self,Some,None};
		pub use core::result::Result::{self,Ok,Err};

		//pub use slice::SliceConcatExt;

		pub use string::{String/*,ToString*/};
		pub use vec::Vec;

		pub use core::slice::SliceExt;
		pub use core::char::CharExt;
		pub use core::str::StrExt;
	}
}

pub mod fmt {
	pub use core::fmt::*;

	pub fn format(args: ::std::fmt::Arguments) -> ::string::String
	{
		let mut output = ::string::String::new();
		let _ = output.write_fmt(args);
		output
	}

}

mod start;

pub mod ffi;

mod heap;

//pub extern crate std_io as io;
extern crate std_io;
pub use std_io as io;

//pub extern crate std_rt as rt;
extern crate std_rt;
pub use std_rt as rt;

//pub extern crate std_sync as sync;
extern crate std_sync;
pub use std_sync as sync;

pub mod error;

pub mod vec;
pub mod string;

