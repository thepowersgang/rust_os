// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/_common.rs
// - Common global definitons
//
// All files have 'use _common::*' as the first line, which imports the names from this module
pub use core::prelude::*;

// - Not in core::prelude, but I like them
pub use core::any::Any;

pub use lib::mem::Box;
pub use lib::vec::Vec;
pub use lib::string::String;
pub use lib::collections::{MutableSeq};
pub use lib::{OptPtr,OptMutPtr};

pub use logging::HexDump;

// vim: ft=rust
