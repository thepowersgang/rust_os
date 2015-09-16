// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/armv7/memory/phys.rs
//! Physical address space managment
//!
//! Handles reference counting and allocation bitmaps
//use prelude::*;

// 1. Reference counts are maintained as a region of address space containing the reference counts
// 2. Bitmap (maybe?) maintained 

pub fn ref_frame(frame_idx: u64) {
	log_error!("TODO: ref_frame #{:#x}", frame_idx);
}
pub fn deref_frame(frame_idx: u64) -> u32 {
	todo!("deref_frame #{:#x}", frame_idx);
}
pub fn get_multiref_count(frame_idx: u64) -> u32 {
	log_warning!("TODO: get_multiref_count frame_idx={:#x}", frame_idx);
	0
}

pub fn mark_free(frame_idx: u64) -> bool {
	todo!("mark_free - frame_idx={:#x}", frame_idx);
}
pub fn mark_used(frame_idx: u64) {
	todo!("mark_used - frame_idx={:#x}", frame_idx);
}


