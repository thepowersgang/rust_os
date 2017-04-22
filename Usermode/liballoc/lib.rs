//
//
//
#![crate_type="rlib"]
#![crate_name="alloc"]
#![feature(lang_items)]	// Allow definition of lang_items
#![feature(const_fn,unsize,coerce_unsized)]
#![feature(unique,nonzero)]
#![feature(box_syntax)]
#![feature(placement_new_protocol)]
#![feature(optin_builtin_traits)]	// For !Send
#![feature(unboxed_closures)]
#![no_std]

#[macro_use]
extern crate syscalls;
#[macro_use]
extern crate macros;

extern crate std_sync as sync;

mod std {
	pub use core::fmt;
	pub use core::iter;
}

pub mod heap;

mod array;
pub mod boxed;

pub mod rc;
pub mod grc;
pub mod raw_vec;

pub use self::array::ArrayAlloc;


pub fn oom() {
	panic!("Out of memory");
}

