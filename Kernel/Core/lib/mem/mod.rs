// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/mem/mod.rs
//! Memory allocation types
pub use self::rc::Rc;
pub use self::arc::Arc;
pub use self::boxed::Box;

pub mod rc;
pub mod arc;
pub mod aref;
pub mod boxed;

#[allow(improper_ctypes)]
extern "C" {
	#[no_mangle]
	/// C's `memset` function, VERY UNSAFE
	pub fn memset(dst: *mut u8, val: u8, count: usize);
}

// vim: ft=rust

