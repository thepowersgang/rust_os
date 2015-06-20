//
//
//
#![crate_type="rlib"]
#![crate_name="std"]
#![feature(no_std,core)]
#![feature(lang_items)]	// Allow definition of lang_items
//#![staged_api]
//#![feature(staged_api)]
#![no_std]

#[macro_use]
extern crate core;
#[macro_use]
extern crate tifflin_syscalls;

use core::prelude::*;

// Raw re-exports from core
pub use core::{option, result};
pub use core::{slice, str};
pub use core::{fmt, iter};
pub use core::{mem};

mod std {
	pub use core::fmt;
	pub use core::iter;
}

/// Prelude
pub mod prelude {
	pub mod v1 {
		//#![stable(feature="rust1", since="1.0.0")]
		pub use core::prelude::*;
		//pub use core::option::Option::{self,Some,None};
		//pub use core::result::Result::{self,Ok,Err};
	}
}

pub mod rt;

