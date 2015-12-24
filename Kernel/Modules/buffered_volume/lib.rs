// "Tifflin" Kernel - Buffered logical volumes
// - By John Hodge (thePowersGang)
//
// Modules/buffered_volume/lib.rs
//! Buffered logical volume wrapper crate
#![feature(type_ascription)]
#![feature(clone_from_slice)]
#![no_std]
use kernel::prelude::*;
use kernel::metadevs::storage::VolumeHandle;
use kernel::metadevs::storage::IoError;
use kernel::sync::{Mutex,RwLock};

#[macro_use]
extern crate kernel;

/// Wraps a kernel `VolumeHandle` and provides a single buffered block for performing partial reads/writes
pub struct BufferedVolume
{
	vh: VolumeHandle,
	/// Read-Write lock to prevent Read-Modify-Write operations from colliding with other ops
	rmw_hold: RwLock<()>,	// Read-write lock to prevent corruption
	/// A single block buffer
	buffer: Mutex< (u64, Box<[u8]>) >,
}

impl BufferedVolume
{
	pub fn new(vol: VolumeHandle) -> Self
	{
		BufferedVolume {
			rmw_hold: RwLock::new( () ),
			buffer: Mutex::new( (!0, ((0 .. vol.block_size()).map(|_| 0).collect(): Vec<_>).into_boxed_slice()) ),
			vh: vol,
		}
	}
	pub fn read_blocks(&self, block: u64, data: &mut [u8]) -> Result<(), IoError>
	{
		let _h = self.rmw_hold.read();
		self.vh.read_blocks(block, data)
	}
	pub fn write_blocks(&self, block: u64, data: &[u8]) -> Result<(), IoError>
	{
		let _h = self.rmw_hold.read();
		self.vh.write_blocks(block, data)
	}
	pub fn block_size(&self) -> usize {
		self.vh.block_size()
	}

	pub fn read_subblock_single(&self, block: u64, offset: usize, data: &mut [u8]) -> Result<(),IoError>
	{
		assert!( offset < self.vh.block_size() );
		let _h = self.rmw_hold.read();
		let mut cache = self.buffer.lock();
		if cache.0 != block {
			cache.0 = block;
			try!(self.vh.read_blocks(block, &mut cache.1));
		}

		data.clone_from_slice( &cache.1[offset..] );
		Ok( () )
	}
	pub fn write_subblock_single(&self, block: u64, offset: usize, data: &[u8]) -> Result<(),IoError>
	{
		// Acquire a write lock to ensure read-modify-write doesn't alter anything
		let _h = self.rmw_hold.write();
		let mut cache = self.buffer.lock();
		if cache.0 != block {
			cache.0 = block;
			try!(self.vh.read_blocks(block, &mut cache.1));
		}
		cache.1[offset..].clone_from_slice( data );
		try!(self.vh.write_blocks(block, &cache.1));
		Ok( () )
	}
}
