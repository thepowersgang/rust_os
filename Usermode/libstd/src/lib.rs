// Tifflin OS - Standard Library (clone)
// - By John Hodge (thePowersGang)
//
// A clone of rust's libstd customised to work correctly on Tifflin
#![crate_type="rlib"]
#![crate_name="std"]
//#![feature(staged_api)]	// stability
#![feature(lang_items)]	// Allow definition of lang_items
#![feature(linkage)]	// Used for low-level runtime
#![feature(core_intrinsics)]
#![feature(box_syntax)]
#![feature(slice_concat_ext)]
#![feature(allocator_api)]
#![feature(allocator_internals)]
#![feature(test,custom_test_frameworks)]	// used for macro import
#![feature(concat_idents,format_args_nl,log_syntax)]
#![feature(alloc_error_handler)]
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
#[allow(deprecated)]
pub use core::{r#try, assert, assert_eq, panic, write, unreachable, unimplemented};
pub use core::{file, line};
//pub use core::{deriving_Debug};


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

// Crate re-exports
pub use alloc::{rc,boxed};
pub use alloc::slice;
pub use alloc::fmt;

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
	pub mod rust_2015 { pub use super::v1::*; }
	pub mod rust_2018 { pub use super::v1::*; }
	pub mod rust_2021 { pub use super::v1::*; }
	pub mod v1 {
		pub use core::marker::{/*Copy,*/Send,Sync,Sized};
		pub use core::ops::{Drop,Fn,FnMut,FnOnce};
		pub use core::mem::drop;
		pub use alloc::boxed::Box;
		pub use borrow::ToOwned;
		//pub use core::clone::Clone;
		//pub use core::cmp::{PartialEq, PartialOrd, Eq, Ord};
		pub use core::convert::{AsRef,AsMut,Into,From};
		//pub use core::default::Default;
		pub use core::iter::{Iterator,Extend,IntoIterator};
		pub use core::iter::{DoubleEndedIterator, ExactSizeIterator};
		
		pub use core::option::Option::{self,Some,None};
		pub use core::result::Result::{self,Ok,Err};

		//pub use slice::SliceConcatExt;

		pub use string::{String,ToString};
		pub use alloc::vec::Vec;

		// Macro imports?
		#[allow(deprecated)]
		pub use core::prelude::v1::{
			Clone,
			Copy,
			Debug,
			Default,
			Eq,
			Hash,
			Ord,
			PartialEq,
			PartialOrd,
			RustcDecodable,
			RustcEncodable,
			bench,
			global_allocator,
			test,
			test_case,
			};
		pub use core::prelude::v1::{
			assert,
			cfg,
			column,
			compile_error,
			concat,
			concat_idents,
			env,
			file,
			format_args,
			format_args_nl,
			include,
			include_bytes,
			include_str,
			line,
			log_syntax,
			module_path,
			option_env,
			stringify,
			//trace_macros,
			derive,
			};
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

pub use core::arch;

pub mod fs;

pub mod error;

pub use alloc::{vec, string, borrow};

pub mod os;

pub mod heap;

