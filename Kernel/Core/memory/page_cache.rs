// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/page_cache.rs
//! Page-based cache controller
//!
//! This module provides a type that controls a region of memory used only for mapping segments of the
//! file/block cache into memory. It does _not_ manage eviction of cache entries from physical memory.
use core::nonzero::NonZero;
use PAGE_SIZE;
use memory::phys::FrameHandle;
use core::sync::atomic::{AtomicPtr, Ordering};
use memory::virt::ProtectionMode;

type AtomicU32 = ::sync::atomic::AtomicValue<u32>;

/// Error returned by mapping functions
pub struct Error;

/// Unique handle to a page in the cache
pub struct CachedPage(NonZero<*mut [u8; PAGE_SIZE]>);
unsafe impl Send for CachedPage {}
unsafe impl Sync for CachedPage {}

const MAX_ENTS: usize = 1024;	// 4MB of active cache entries.

/// Actual cache structure
pub struct PageCache
{
	avail_ents: ::sync::Semaphore,
	bitmap: [AtomicU32; MAX_ENTS / 32],
	cache_start: AtomicPtr<Page>,
}
unsafe impl Send for PageCache {}
unsafe impl Sync for PageCache {}

struct Page([u8; PAGE_SIZE]);

/// Global page cache instance, use this to access the cache.
pub static S_PAGE_CACHE: PageCache = PageCache::new();

pub fn init()
{
	S_PAGE_CACHE.cache_start.store( super::bump_region::delegate(MAX_ENTS).expect("page_cache init") as *mut Page, Ordering::Release ); 
}

impl PageCache
{
	const fn new() -> PageCache
	{
		PageCache {
			avail_ents: ::sync::Semaphore::new( MAX_ENTS as isize, MAX_ENTS as isize ),
			bitmap: [
				AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),  AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),
				AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),  AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),
				AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),  AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),
				AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),  AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0),
				],
			cache_start: AtomicPtr::new(0 as *mut _),
		}
	}

	fn addr(&self, idx: usize) -> *mut Page {
		let base = self.cache_start.load(Ordering::Acquire);
		assert!( ! base.is_null() );
		(base as usize + idx * PAGE_SIZE) as *mut Page
	}

	/// Map the provided physical frame into virtual memory and return a handle to it
	/// 
	// TODO: This should be unsafe, as passing the same FrameHandle twice will induce aliasing
	pub fn map(&self, frame_handle: &FrameHandle) -> Result<CachedPage, Error>
	{
		self.avail_ents.acquire();

		for (blk, e) in self.bitmap.iter().enumerate()
		{
			loop
			{
				let cur = e.load(Ordering::Relaxed);
				if cur == !0 {
					break;
				}

				let i = (!cur).trailing_zeros() as usize;
				
				if cur == e.compare_and_swap(cur, cur | (1 << i), Ordering::Acquire) {
					let addr = self.addr( blk * 32 + i );
					assert!( !addr.is_null() );
					// SAFE: Assuming that we're passed an unaliased handle. Address is non-zero
					unsafe {
						::memory::virt::map(addr as *mut (), frame_handle.clone().into_addr(), ProtectionMode::KernelRW);
						return Ok( CachedPage(NonZero::new(addr as *mut _)) );
					}
				}
			}
		}

		todo!("Is this even possible?");
	}

	/// Allocate a new frame and place it in the cache
	pub fn create(&self) -> Result<CachedPage, Error>
	{
		todo!("PageCache::create");
	}
}



impl CachedPage
{
	pub fn get_frame_handle(&self) -> FrameHandle {
		todo!("CachedPage::get_frame_handle");
	}
}
impl ::core::ops::Drop for CachedPage
{
	fn drop(&mut self) {
		todo!("CachedPage::drop()");
	}
}


