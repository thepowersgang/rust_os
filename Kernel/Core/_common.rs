// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/_common.rs
// - Common global definitons
//
// All files have 'use _common::*' as the first line, which imports the names from this module
pub use core::iter::{range,Iterator};
pub use core::collections::Collection;
pub use core::slice::{Slice,ImmutableSlice,MutableSlice};
pub use core::default::Default;
pub use core::option::{Option,Some,None};
pub use core::ops::{Deref,DerefMut};
pub use core::cmp::PartialEq;

// vim: ft=rust
