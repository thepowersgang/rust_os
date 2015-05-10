// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/prelude.rs
/// Common global definitons
///
/// All files have 'use prelude::*' as the first line, which imports the names from this module
// Recreate std::prelude
pub use core::prelude::*;

pub use lib::mem::Box;
//pub use lib::borrow::ToOwned;
pub use lib::vec::Vec;
pub use lib::string::String;


// - Not in core::prelude, but I like them
pub use core::any::Any;

pub use lib::collections::{MutableSeq};
pub use logging::HexDump;

// vim: ft=rust
