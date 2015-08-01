// Tifflin OS - Standard Library (clone)
// - By John Hodge (thePowersGang)
//
// A clone of rust's libstd customised to work correctly on Tifflin
#![crate_type="rlib"]
#![crate_name="std"]
#![feature(no_std,core,core_prelude)]
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
extern crate core;
#[macro_use]
extern crate syscalls;
#[macro_use]
extern crate macros;

//extern crate loader;

//use core::prelude::*;

// Raw re-exports from core
pub use core::{option, result};
pub use core::{slice, str, ptr};
pub use core::{fmt, iter, clone};
pub use core::{mem, cmp, ops};
pub use core::{default};
pub use core::convert;
pub use core::intrinsics;
pub use core::marker;

mod std {
	pub use core::{option, result};
	pub use core::fmt;
	pub use core::iter;
	pub use core::{mem, cmp, ops};
}

/// Prelude
pub mod prelude {
	pub mod v1 {
		pub use core::prelude::*;
		//pub use core::option::Option::{self,Some,None};
		//pub use core::result::Result::{self,Ok,Err};
		pub use string::String;
		pub use vec::Vec;
	}
}

fn type_name<T: ?::core::marker::Sized>() -> &'static str { unsafe { ::core::intrinsics::type_name::<T>() } }
macro_rules! type_name {
	($t:ty) => ( $crate::type_name::<$t>() );
}
macro_rules! todo
{
	( $s:expr ) => ( panic!( concat!("TODO: ",$s) ) );
	( $s:expr, $($v:tt)* ) => ( panic!( concat!("TODO: ",$s), $($v)* ) );
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

