// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/armv8/memory/phys.rs
//! Physical address space managment
//!
//! Handles reference counting and allocation bitmaps
//use prelude::*;
//use arch::imp::memory::addresses::{PMEMREF_BASE,PMEMREF_END/*,PMEMBM_BASE,PMEMBM_END*/};
use sync::{RwLock,AtomicU32};
use core::sync::atomic::Ordering;
use memory::page_array::PageArray;

//static S_REFCOUNT_ARRAY: RwLock<PageArray<AtomicU32>> = RwLock::new( PageArray::new(PMEMREF_BASE, PMEMREF_END) );

pub fn ref_frame(frame_idx: u64) {
	with_ref_alloc( frame_idx, |r| r.fetch_add(1, Ordering::Acquire) );
}
pub fn deref_frame(frame_idx: u64) -> u32 {
	with_ref(frame_idx, |r|
		if r.load(Ordering::Relaxed) != 0 {
			r.fetch_sub(1, Ordering::Release)
		}
		else {
			0
		}
		).unwrap_or(0)
}
pub fn get_multiref_count(frame_idx: u64) -> u32 {
	with_ref( frame_idx, |r| r.load(Ordering::Relaxed) ).unwrap_or(0)
}

pub fn mark_free(frame_idx: u64) -> bool {
	log_warning!("TODO: mark_free - frame_idx={:#x} ({:#x})", frame_idx, frame_idx*::PAGE_SIZE as u64);
	// HACK: Assume it was used
	true
}
pub fn mark_used(frame_idx: u64) {
	log_warning!("TODO: mark_used - frame_idx={:#x} ({:#x})", frame_idx, frame_idx*::PAGE_SIZE as u64);
}


fn with_ref<U, F: FnOnce(&AtomicU32)->U>(frame_idx: u64, fcn: F) -> Option<U>
{
	todo!("");
	//S_REFCOUNT_ARRAY.read().get(frame_idx as usize).map(fcn)
}
fn with_ref_alloc<U, F: FnOnce(&AtomicU32)->U>(frame_idx: u64, fcn: F) -> U
{
	todo!("");
	//let mut lh = S_REFCOUNT_ARRAY.write();
	//fcn( lh.get_alloc(frame_idx as usize) )
}

