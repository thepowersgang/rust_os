// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/block_cache/lib.rs
//! System-gobal block cache (for filesystem metadata)
#![no_std]

use kernel::prelude::*;
use kernel::PAGE_SIZE;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use kernel::metadevs::storage::{VolumeHandle,IoError};
use kernel::sync::{RwLock,rwlock};
use kernel::sync::mutex::LazyMutex;

// NOTES:
// - Handles wrap logical volume handles
// - Presents:
//  > read/write (unbuffered)
//  > read_inner/get/edit (buffered)
//
// - The global cache is registered with the PMM as a source of reclaimable memory

#[macro_use]
extern crate kernel;

/// A handle into the cache corresponding to a logical volume
pub struct CacheHandle
{
	vh: VolumeHandle,
}

/// A handle to a block in the cache
pub struct CachedBlockHandle<'a>(MetaBlockHandle<'a>);

struct MetaBlockHandle<'a>(&'a CachedBlock);

/// Global cache structure
#[derive(Default)]
struct Cache
{
	map: ::kernel::lib::VecMap< (usize, u64), Box<CachedBlock> >,
}

struct CachedBlock
{
	// Constant:
	index: u64,
	block_paddr: ::kernel::memory::phys::FrameHandle,

	reference_count: AtomicUsize,
	last_access: ::kernel::time::CacheTimer,
	is_dirty: AtomicBool,

	mapping: RwLock<Option<::kernel::memory::page_cache::CachedPage>>,
}


static S_BLOCK_CACHE: LazyMutex<Cache> = LazyMutex::new();
//static S_BLOCK_CACHE: Mutex<Cache> = Mutex::new(Cache {
//	map: ::kernel::lib::VecMap::new(),
//	});

impl CacheHandle
{
	pub fn new(vol: VolumeHandle) -> CacheHandle
	{
		if vol.block_size() > ::kernel::PAGE_SIZE {
			todo!("Support caching volumes with block sizes > page size");
		}

		CacheHandle {
			vh: vol,
			}
	}

	pub fn blocks_per_page(&self) -> u64 {
		(PAGE_SIZE / self.vh.block_size()) as u64
	}
}

/// Unbuffered IO methods. These just directly read/write from the volume.
impl CacheHandle
{
	pub fn name(&self) -> &str {
		self.vh.name()
	}
	pub fn idx(&self) -> usize {
		self.vh.idx()
	}
	pub fn block_size(&self) -> usize {
		self.vh.block_size()
	}
	pub async fn read_blocks(&self, block: u64, data: &mut [u8]) -> Result<(), IoError>
	{
		self.vh.read_blocks(block, data).await
	}
	pub async fn write_blocks(&self, block: u64, data: &[u8]) -> Result<(), IoError>
	{
		self.vh.write_blocks(block, data).await
	}
}

/// Cached accesses
impl CacheHandle
{
	async fn get_block_meta(&self, block: u64) -> Result<MetaBlockHandle<'_>, IoError>
	{
		let cache_block = block - block % self.blocks_per_page();
		let handle = {
			use kernel::lib::vec_map::Entry;
			let mut lh = S_BLOCK_CACHE.lock_init(|| Default::default());
			let handle = match lh.map.entry( (self.vh.idx(), cache_block) )
				{
				Entry::Occupied(v) => v.into_mut().borrow(),
				Entry::Vacant(v) => v.insert( Box::new( CachedBlock::new(&self.vh, cache_block).await? ) ).borrow(),
				};
			// SAFE: 1. The internal data is boxed, 2. The box won't be dropped while a borrow exists.
			unsafe { ::core::mem::transmute::<MetaBlockHandle, MetaBlockHandle>(handle) }
			};
		Ok(handle)
	}

	/// Obtain a handle to a cached block.
	/// NOTE: The returned handle will point to the start of the cache block, which may be larger than the disk block. Remember to check the returned block index.
	pub async fn get_block(&self, block: u64) -> Result<CachedBlockHandle<'_>, IoError>
	{
		Ok( self.get_block_meta(block).await?.into_ro() )
	}

	/// Read out of a cached block
	pub async fn read_inner(&self, block: u64, offset: usize, data: &mut [u8]) -> Result<(),IoError>
	{
		let cached_block = self.get_block(block).await?;
		let blk_ofs = (block - cached_block.index()) as usize * self.block_size();

		if offset >= self.block_size() {
			return Err(IoError::InvalidParameter);
		}
		if offset >= self.block_size() - blk_ofs {
			return Err(IoError::InvalidParameter);
		}
		assert!(data.len() <= self.block_size() - offset);
		let bytes = data.len();
		data.clone_from_slice( &cached_block.data()[blk_ofs + offset .. ][ .. bytes] );
		Ok( () )
	}
	/// Write into a cached block
	pub async fn write_inner(&self, block: u64, offset: usize, data: &[u8]) -> Result<(), IoError>
	{
		let cached_block = self.get_block_meta(block).await?;
		let blk_ofs = (block - cached_block.index()) as usize * self.block_size();

		if offset >= self.block_size() {
			return Err(IoError::InvalidParameter);
		}
		if blk_ofs + offset >= self.block_size() {
			return Err(IoError::InvalidParameter);
		}

		cached_block.edit(|block_data| {
			block_data[blk_ofs + offset ..].clone_from_slice( data );
			Ok( () )
			})
	}
	/// Edit block
	pub async fn edit<F: FnOnce(&mut [u8])->R,R>(&self, block: u64, count: usize, f: F) -> Result<R, IoError>
	{
		let cached_block = self.get_block_meta(block).await?;
		let blk_ofs = (block - cached_block.index()) as usize * self.block_size();

		if (block - cached_block.index()) as usize + count > self.blocks_per_page() as usize {
			return Err(IoError::InvalidParameter);
		}

		let rv = cached_block.edit(|block_data| {
			f( &mut block_data[blk_ofs ..][ .. count * self.block_size()] )
			});

		cached_block.0.flush(&self.vh).await?;

		Ok( rv )
	}
}

fn map_cached_frame(frame: &::kernel::memory::phys::FrameHandle) -> ::kernel::memory::page_cache::CachedPage
{
	// TODO: If this returns that there's no free mappings, go and steal one from within the cache
	// - Or just do a GC pass, then try again.
	::kernel::memory::page_cache::S_PAGE_CACHE.map(frame).expect("TODO: OOM in CachedBlock::borrow")
}

// --------------------------------------------------------------------
impl CachedBlock
{
	async fn new(vol: &VolumeHandle, first_block: u64) -> Result<CachedBlock, IoError>
	{
		let mut mapping = ::kernel::memory::page_cache::S_PAGE_CACHE.create().map_err(|_| IoError::Unknown("OOM"))?;

		// TODO: Defer disk read until after the cache entry is created
		vol.read_blocks(first_block, mapping.data_mut()).await?;
		
		Ok(CachedBlock {
			index: first_block,
			block_paddr: mapping.get_frame_handle(),
			reference_count: AtomicUsize::new(0),

			last_access: Default::default(),
			is_dirty: AtomicBool::new(false),
			mapping: RwLock::new(Some(mapping)),
			})
	}
	
	/// Write a modified block back to disk
	async fn flush(&self, vol: &VolumeHandle) -> Result<(), IoError>
	{
		let lh = self.mapping.read();
		if self.is_dirty.swap(false, Ordering::Acquire)
		{
			vol.write_blocks(self.index, lh.as_ref().expect("CachedBlock::flush - None mapping").data()).await?;
		}
		Ok( () )
	}
	
	fn borrow(&self) -> MetaBlockHandle {

		if self.mapping.read().is_none()
		{
			let mut lh = self.mapping.write();
			if lh.is_none() {
				*lh = Some( map_cached_frame(&self.block_paddr) );
			}
		}

		self.reference_count.fetch_add(1, Ordering::Acquire);
		self.last_access.bump();

		MetaBlockHandle(self)
	}
}

impl<'a> MetaBlockHandle<'a>
{
	pub fn index(&self) -> u64 {
		self.0.index
	}

	pub fn edit<F: FnOnce(&mut [u8])->R, R>(&self, f: F) -> R {
		let mut lh = self.0.mapping.write();
		let dataptr = lh.as_mut().expect("CachedBlock mapping is None").data_mut();
		self.0.is_dirty.store(true, Ordering::Relaxed);
		f(dataptr)
	}

	pub fn into_ro(self) -> CachedBlockHandle<'a> {
		let read_handle = self.0.mapping.read();
		::core::mem::forget(read_handle);
		CachedBlockHandle( self/*, read_handle*/ )
	}
}
impl<'a> ::core::ops::Drop for MetaBlockHandle<'a>
{
	fn drop(&mut self)
	{
		// TODO: Clearing this when the refcount reaches zero will lead to a lot of mapping/unmapping
		// - Maybe keep a local queue or run a purge every now and then. Using a LRU list could work.
		/*
		if self.0.reference_count.fetch_sub(1, Ordering::Release) == 1
		{
			let mut wh = self.0.mapping.write();
			*wh = None;
		}
		*/
	}
}

impl<'a> CachedBlockHandle<'a>
{
	fn block(&self) -> &CachedBlock {
		self.0 .0
	}

	pub fn index(&self) -> u64 {
		self.block().index
	}

	pub fn data(&self) -> &[u8] {
		// SAFE: Read handle is constructed from a read-locked RwLock, and forgotten soon after
		let rawptr: *const [u8] = unsafe {
			let rh = rwlock::Read::from_raw(&self.block().mapping);
			let p: *const [u8] = rh.as_ref().expect("None mapping").data();
			::core::mem::forget(rh);
			p
			};
		// SAFE: While this type exists, the mapping should not be invalidated or mutated
		unsafe { &*rawptr }
	}
}
impl<'a> Drop for CachedBlockHandle<'a>
{
	fn drop(&mut self)
	{
		// SAFE: Read hanle is constructed from a read-locked RwLock
		let _ = unsafe { rwlock::Read::from_raw(&self.block().mapping) };
	}
}

