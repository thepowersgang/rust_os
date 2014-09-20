// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/_common.rs
// - Common global definitons
//
// All files have 'use _common::*' as the first line, which imports the names from this module
pub use core::iter::{range,range_step};
pub use core::iter::{Iterator,DoubleEndedIterator};
pub use core::collections::{Collection};
pub use core::slice::{Slice,ImmutableSlice,MutableSlice};
pub use core::str::StrSlice;
pub use core::default::Default;
pub use core::option::{Option,Some,None};
pub use core::result::{Result,Ok,Err};
pub use core::ops::{Drop,Deref,DerefMut};
pub use core::cmp::PartialEq;
pub use core::num::Int;

pub use lib::vec::Vec;
pub use lib::clone::Clone;
pub use lib::collections::{MutableSeq};

// vim: ft=rust
