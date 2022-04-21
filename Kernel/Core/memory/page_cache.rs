// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/memory/page_cache.rs
//! Page-based cache controller
//!
//! This module provides a type that controls a region of memory used only for mapping segments of the
//! file/block cache into memory. It does _not_ manage eviction of cache entries from physical memory.
//
// TODO: Should this module handle a LRU cache of mappings (not actually unmap until needed)
use core::ptr::NonNull;
use crate::PAGE_SIZE;
use crate::memory::phys::FrameHandle;
use core::sync::atomic::{Ordering, AtomicPtr, AtomicU32};
use crate::memory::virt::ProtectionMode;

/// Error returned by mapping functions
#[derive(Debug)]
pub struct Error;
impl_from! {
	From<crate::memory::virt::MapError>(_v) for Error {
		Error
	}
}

/// NonNull handle to a page in the cache
pub struct CachedPage(NonNull<Page>);
unsafe impl Send for CachedPage {}
unsafe impl Sync for CachedPage {}

const MAX_ENTS: usize = 1024;	// 4MB of active cache entries.

/// Actual cache structure
pub struct PageCache
{
	avail_ents: crate::sync::Semaphore,
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
	S_PAGE_CACHE.cache_start.store( super::bump_region::delegate(MAX_ENTS).expect("page_cache init") as *mut Page, Ordering::Relaxed ); 
	log_debug!("init: S_PAGE_CACHE.cache_start={:p}", S_PAGE_CACHE.cache_start.load(Ordering::Relaxed));
}

impl PageCache
{
	const fn new() -> PageCache
	{
		PageCache {
			avail_ents: crate::sync::Semaphore::new( MAX_ENTS as isize, MAX_ENTS as isize ),
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
		assert!( ! base.is_null(), "Cache not yet initialised? self={:p}", self );
		(base as usize + idx * PAGE_SIZE) as *mut Page
	}

	fn get_free_ent<F,R>(&self, cb: F) -> Result<R, Error>
	where
		F: FnOnce(usize) -> Result<R, Error>
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
				
				if let Ok(_) = e.compare_exchange(cur, cur | (1 << i), Ordering::Acquire, Ordering::Relaxed) {
					return cb( blk * 32 + i );
				}
			}
		}

		todo!("Is this even possible?");
	}

	/// Map the provided physical frame into virtual memory and return a handle to it
	/// 
	// TODO: This should be unsafe, as passing the same FrameHandle twice will induce aliasing
	pub fn map(&self, frame_handle: &FrameHandle) -> Result<CachedPage, Error>
	{
		self.get_free_ent(|idx| {
			let addr = self.addr( idx );
			assert!( !addr.is_null() );
			// SAFE: Assuming that we're passed an unaliased handle. Address is non-zero
			unsafe {
				crate::memory::virt::map(addr as *mut (), frame_handle.clone().into_addr(), ProtectionMode::KernelRW);
				Ok( CachedPage(NonNull::new_unchecked(addr as *mut _)) )
			}
			})
	}

	/// Allocate a new frame and place it in the cache
	pub fn create(&self) -> Result<CachedPage, Error>
	{
		self.get_free_ent(|idx| {
			let addr = self.addr(idx);
			#[cfg(not(feature="test"))]
			crate::memory::virt::allocate(addr as *mut (), 1)?;
			// SAFE: Non-null pointer
			Ok( CachedPage(unsafe { NonNull::new_unchecked(addr as *mut _) }) )
			})
	}


	fn release(&self, addr: *mut Page)
	{
		assert!(addr as usize % crate::PAGE_SIZE == 0);
		let base = self.addr(0);
		assert!(addr as usize >= base as usize);
		let idx = (addr as usize - base as usize) / crate::PAGE_SIZE;
		assert!(idx < MAX_ENTS);

		// SAFE: Internally only called on drop of handle
		unsafe {
			crate::memory::virt::unmap(addr as *mut (), 1);
		}

		let e = &self.bitmap[idx / 32];
		let mask: u32 = 1 << (idx % 32);
		loop
		{
			let cur = e.load(Ordering::Acquire);
			if let Ok(_) = e.compare_exchange(cur, cur & !mask, Ordering::Release, Ordering::Relaxed) {
				break ;
			}
		}
		self.avail_ents.release();
	}
}



impl CachedPage
{
	pub fn get_frame_handle(&self) -> FrameHandle {
		// TODO: Is this actually safe? It would allow aliasing if someone maps this FrameHandle
		// SAFE: Physical address is valid
		unsafe {
			FrameHandle::from_addr( crate::memory::virt::get_phys(self.0.as_ptr()) )
		}
	}
	pub fn data(&self) -> &[u8] {
		// SAFE: Owned and valid
		unsafe { &self.0.as_ref().0 }
	}
	pub fn data_mut(&mut self) -> &mut [u8] {
		// SAFE: Owned and valid
		unsafe { &mut self.0.as_mut().0 }
	}
}
impl ::core::ops::Drop for CachedPage
{
	fn drop(&mut self) {
		S_PAGE_CACHE.release( self.0.as_ptr() );
	}
}


