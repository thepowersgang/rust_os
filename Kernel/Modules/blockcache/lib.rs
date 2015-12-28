// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/bloccache/lib.rs
//! Small block cache for use by filesystem drivers
//!
//! Mostly intended to reduce churn on metadata blocks.
#![no_std]
#[macro_use] extern crate kernel;
#[allow(unused_imports)]
use kernel::prelude::*;

use kernel::lib::mem::Arc;

/// A simple Least-Recently-Used block cache, with six slots
///
/// Has an internal mutex, and uses Arc as the buffer type.
pub struct BlockCache//<#count>
{
	lru_blocks: ::kernel::sync::Mutex< [Option<CachedBlock>; 8/*#count*/] >,
}

struct CachedBlock
{
	lba: u32,
	time: ::kernel::time::TickCount,
	data: Arc<[u8]>,
}

impl BlockCache
{
	/// Construct a new cache instance
	pub fn new() -> Self {
		BlockCache {
			lru_blocks: Default::default(),
		}
	}
	
	/// Obtain a block via the cache, calling the provided closure if the block is not present
	pub fn get<E, F: FnOnce(u32)->Result<Arc<[u8]>,E>>(&self, lba: u32, populate: F) -> Result<Arc<[u8]>,E>
	{
		let mut lh = self.lru_blocks.lock();
		let (mut oldest_i, mut oldest_ts) = (0,!0);
		// Search cache for the sector (and look for a suitable location)
		for (i,e) in lh.iter_mut().enumerate()
		{
			match *e
			{
			Some(ref mut e) => {
				if e.time < oldest_ts {
					oldest_i = i;
					oldest_ts = e.time;
				}
				// If the LBA matches, update the timestamp and return a handle
				if e.lba == lba {
					//log_trace!("Hit: {}", lba);
					e.time = ::kernel::time::ticks();
					return Ok(e.data.clone());
				}
				},
			None => {
				oldest_i = i;
				oldest_ts = 0;
				},
			}
		}
		
		// If the block wasn't in the cache, read and cache it
		log_trace!("Miss: {}", lba);
		let data = try!( populate(lba) );
		
		lh[oldest_i] = Some(CachedBlock {
			time: ::kernel::time::ticks(),
			lba: lba,
			data: data.clone(),
			});
		
		Ok( data )
	}
}

