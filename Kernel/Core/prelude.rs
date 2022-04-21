// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/prelude.rs
/// Common global definitons
///
/// All files have 'use prelude::*' as the first line, which imports the names from this module
// Recreate std::prelude
pub use core::prelude::*;

pub use crate::lib::mem::boxed::Box;

//pub use lib::borrow::ToOwned;
pub use crate::lib::vec::Vec;
pub use crate::lib::string::String;


// - Not in core::prelude, but I like them
pub use core::any::Any;

pub use crate::lib::collections::{MutableSeq};
pub use crate::logging::HexDump;

// vim: ft=rust
