// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/block_cache/lib.rs
//! System-gobal block cache (for filesystem metadata)
#![no_std]

#![feature(const_fn)]
#![feature(nonzero)]
#![feature(clone_from_slice)]

use kernel::prelude::*;
use kernel::PAGE_SIZE;
use core::sync::atomic::{AtomicUsize, AtomicBool, Ordering};
use kernel::metadevs::storage::{VolumeHandle,IoError};
use kernel::sync::Mutex;
use kernel::sync::mutex::LazyMutex;
use kernel::sync::Spinlock;

// NOTES:
// - Handles wrap logical volume handles
// - Presents:
//  > read/write (unbuffered)
//  > read_inner/get/edit (buffered)
//
// - The global cache is registerd with the PMM as a source of reclaimable memory

#[macro_use]
extern crate kernel;

/// A handle into the cache corresponding to a logical volume
pub struct CacheHandle
{
	vh: VolumeHandle,
}

/// A handle to a block in the cache
pub struct CachedBlockHandle(::core::nonzero::NonZero<*const CachedBlock>);


/// Global cache structure
#[derive(Default)]
struct Cache
{
	map: ::kernel::lib::VecMap< (usize, u64), Box<CachedBlock> >,
}

struct CachedBlock
{
	index: u64,
	last_access: Spinlock<::kernel::time::TickCount>,
	is_dirty: AtomicBool,
	reference_count: AtomicUsize,
	block_paddr: ::kernel::memory::phys::FrameHandle,
	mapping: Mutex<Option<::kernel::memory::page_cache::CachedPage>>,	// TODO: A mutex is pretty heavy...
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
	pub fn read_blocks(&self, block: u64, data: &mut [u8]) -> Result<(), IoError>
	{
		self.vh.read_blocks(block, data)
	}
	pub fn write_blocks(&self, block: u64, data: &[u8]) -> Result<(), IoError>
	{
		self.vh.write_blocks(block, data)
	}
}

/// Cached accesses
impl CacheHandle
{
	/// Obtain a handle to a cached block.
	/// NOTE: The returned handle will point to the start of the cache block, which may be larger than the disk block. Remember to check the returned block index.
	pub fn get_block(&self, block: u64) -> Result<CachedBlockHandle, IoError>
	{
		let cache_block = block - block % self.blocks_per_page();
		let handle = {
			use kernel::lib::vec_map::Entry;
			let mut lh = S_BLOCK_CACHE.lock_init(|| Default::default());
			match lh.map.entry( (self.vh.idx(), cache_block) )
			{
			Entry::Occupied(v) => v.into_mut().borrow(),
			Entry::Vacant(v) => v.insert( Box::new( try!(CachedBlock::new(&self.vh, cache_block)) ) ).borrow(),
			}
			};
		Ok( handle )
	}

	/// Read out of a cached block
	pub fn read_inner(&self, block: u64, offset: usize, data: &mut [u8]) -> Result<(),IoError>
	{
		let cached_block = try!(self.get_block(block));
		let blk_ofs = (block - cached_block.index()) as usize * self.block_size();

		// TODO: Check ranges to avoid panics
		data.clone_from_slice( &cached_block.data()[blk_ofs + offset .. ] );
		Ok( () )
	}
	/// Write into a cached block
	pub fn write_inner(&self, block: u64, offset: usize, data: &[u8]) -> Result<(), IoError>
	{
		todo!("");
	}
}


// --------------------------------------------------------------------
impl CachedBlock
{
	fn new(vol: &VolumeHandle, first_block: u64) -> Result<CachedBlock, IoError>
	{
		let mut mapping = try!(::kernel::memory::page_cache::S_PAGE_CACHE.create().map_err(|_| IoError::Unknown("OOM")));

		try!( vol.read_blocks(first_block, mapping.data_mut()) );
		//log_debug!("Loaded block={}", first_block);
		//::kernel::logging::hex_dump("", mapping.data());
		
		Ok(CachedBlock {
			index: first_block,
			reference_count: AtomicUsize::new(0),
			last_access: Default::default(),
			is_dirty: AtomicBool::new(false),
			block_paddr: mapping.get_frame_handle(),
			mapping: Mutex::new( Some(mapping) ),
			})
	}

	fn borrow(&self) -> CachedBlockHandle {
		self.reference_count.fetch_add(1, Ordering::Acquire);
		let mut lh = self.mapping.lock();
		if lh.is_none() {
			*lh = Some( ::kernel::memory::page_cache::S_PAGE_CACHE.map(&self.block_paddr).expect("TODO: OOM in CachedBlock::borrow") );
		}

		*self.last_access.lock() = ::kernel::time::ticks();

		// SAFE: Non-zero value passed
		CachedBlockHandle( unsafe { ::core::nonzero::NonZero::new(self) } )
	}
	fn release_borrow(&self) {
		if self.reference_count.fetch_sub(1, Ordering::Release) == 1 {
			// TODO: Clearing this when the refcount reaches zero will lead to a lot of mapping/unmapping
			let mut lh = self.mapping.lock();
			if self.reference_count.load(Ordering::SeqCst) == 0 {
				*lh = None;
			}
		}
	}
}


impl CachedBlockHandle
{
	fn block(&self) -> &CachedBlock {
		// SAFE: While this handle exists, the reference count is positive, hence the pointer should be valid
		unsafe { &**self.0 }
	}

	pub fn index(&self) -> u64 {
		self.block().index
	}

	pub fn data(&self) -> &[u8] {
		let raw: *const [u8] = self.block().mapping.lock().as_ref().expect("None mapping").data();
		// SAFE: While this type exists, the mapping should not be invalidated or mutated
		unsafe { &*raw }
	}
}
impl Drop for CachedBlockHandle
{
	fn drop(&mut self)
	{
		self.block().release_borrow();
	}
}

