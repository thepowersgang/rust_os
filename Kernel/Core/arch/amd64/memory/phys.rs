// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/memory/phys.rs
//! Physical address space managment
//!
//! Handles reference counting and allocation bitmaps
use arch::imp::memory::addresses::{PMEMREF_BASE,PMEMREF_END};
use sync::RwLock;
use core::sync::atomic::{Ordering};
use sync::AtomicU32;

// 1. Reference counts are maintained as a region of address space containing the reference counts
// 2. Bitmap (maybe?) maintained 

static S_REFCOUNT_ARRAY: RwLock<PageArray<AtomicU32>> = RwLock::new( PageArray::new(PMEMREF_BASE, PMEMREF_END) );

/// Calls the provided closure with a borrow of the reference count for the specified frame
fn with_ref<U, F: FnOnce(&AtomicU32)->U>(frame_idx: u64, fcn: F) -> Option<U>
{
	S_REFCOUNT_ARRAY.read().get(frame_idx as usize).map(fcn)
}
/// Calls the provided closure with a reference to the specified frame's reference count (allocating if required)
fn with_ref_alloc<U, F: FnOnce(&AtomicU32)->U>(frame_idx: u64, fcn: F) -> U
{
	let mut lh = S_REFCOUNT_ARRAY.write();
	fcn( lh.get_alloc(frame_idx as usize) )
}


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
	// Don't really use this though...
	log_notice!("TODO: mark_free - frame_idx={:#x}", frame_idx);
	true
}
pub fn mark_used(frame_idx: u64) {
	log_notice!("TODO: mark_used - frame_idx={:#x}", frame_idx);
}


struct PageArray<T>
{
	start: *mut T,
	//capacity: usize,
	end: *mut T,
}
unsafe impl<T: Send> Send for PageArray<T> {}
unsafe impl<T: Sync> Sync for PageArray<T> {}
impl<T> PageArray<T>
{
	pub const fn new(start: usize, end: usize) -> PageArray<T> {
		//assert_eq!( PAGE_SIZE % ::core::mem::size_of::<T>() == 0, "Creating a PageArray<{}> isn't possible - doesn't fit evenly", type_name!(T) );
		PageArray {
			start: start as *mut _,
			end: end as *mut _,
			}
	}

	pub fn capacity(&self) -> usize {
		(self.end as usize - self.start as usize) / ::core::mem::size_of::<T>()
	}

	pub fn get(&self, idx: usize) -> Option<&T> {
		if idx > self.capacity() {
			None
		}
		else {
			// SAFE: Pointer is in range, and validity is checked. Lifetime valid due to self owning area
			unsafe {
				let ptr = self.start.offset(idx as isize);
				if ::memory::virt::is_reserved(ptr) {
					Some(&*ptr)
				}
				else {
					None
				}
			}
		}
	}

	pub fn get_alloc(&mut self, idx: usize) -> &mut T
	where
		T: Default
	{
		let per_page = ::PAGE_SIZE / ::core::mem::size_of::<T>();
		let pgidx = idx / per_page;
		let pgofs = idx % per_page;
		let page = (self.start as usize + pgidx * ::PAGE_SIZE) as *mut T;
		if ! ::memory::virt::is_reserved( page ) {
			::memory::virt::allocate( page as *mut (), 1 );

			// SAFE: Newly allocated, and nothing valid in it
			unsafe {
				for i in 0 .. per_page {
					let p = page.offset(i as isize);
					::core::ptr::write(p, Default::default());
				}
			}
		}
		// SAFE: Valid, and lifetimes keep it valid
		unsafe {
			&mut *page.offset(pgofs as isize)
		}
	}
}

