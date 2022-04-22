
use kernel::prelude::*;
use kernel::metadevs::storage;
use crate::interface::Interface;
use crate::queue::{Queue,Buffer};

#[allow(dead_code)]
mod defs {
pub const VIRTIO_BLK_F_RO	: u32 = 1 << 5;
// TODO: Other feature flags

pub const VIRTIO_BLK_T_IN    	: u32 = 0;
pub const VIRTIO_BLK_T_OUT  	: u32 = 1;
pub const VIRTIO_BLK_T_SCSI_CMD	: u32 = 2;
pub const VIRTIO_BLK_T_SCSI_CMD_OUT	: u32 = 3;
pub const VIRTIO_BLK_T_FLUSH	: u32 = 4;
pub const VIRTIO_BLK_T_FLUSH_OUT: u32 = 5;
pub const VIRTIO_BLK_T_BARRIER	: u32 = 0x8000_0000;
}
use self::defs::*;

pub struct BlockDevice
{
	_pv_handle: storage::PhysicalVolumeReg,
}

struct Volume<I: Interface>
{
	interface: I,
	capacity: u64,
	requestq: Queue,
}

impl BlockDevice
{
	pub fn new<T: Interface+Send+'static>(mut int: T) -> Self {
		// SAFE: Readable registers
		let capacity = unsafe { int.cfg_read_32(0) as u64 | ((int.cfg_read_32(4) as u64) << 32) };
		log_debug!("Block Device: {}", storage::SizePrinter(capacity * 512));

		let requestq = int.get_queue(0, 0).expect("Queue #0 'requestq' missing on virtio block device");
	
		let features = int.negotiate_features( VIRTIO_BLK_F_RO );
		if features & VIRTIO_BLK_F_RO != 0 {
			// TODO: Need a way of indicating to the upper layers that a volume is read-only
		}
		int.set_driver_ok();

		let mut vol = Box::new(Volume {
			requestq: requestq,
			capacity: capacity,
			interface: int,
			});

		vol.interface.bind_interrupt(vol.requestq.check_interrupt_fn());

		BlockDevice {
			_pv_handle: storage::register_pv(vol),
			}
	}
}
impl ::kernel::device_manager::DriverInstance for BlockDevice {
}

#[repr(C)]
struct VirtioBlockReq
{
	type_: u32,
	ioprio: u32,
	sector: u64,
}
unsafe impl ::kernel::lib::POD for VirtioBlockReq {}

const BLOCK_SIZE: usize = 512;
impl<I: Interface+Send+'static> storage::PhysicalVolume for Volume<I>
{
	fn name(&self) -> &str { "virtio0" }
	fn blocksize(&self) -> usize { BLOCK_SIZE }
	fn capacity(&self) -> Option<u64> { Some(self.capacity) }
	
	fn read<'a>(&'a self, prio: u8, idx: u64, num: usize, dst: &'a mut [u8]) -> storage::AsyncIoResult<'a,usize>
	{
		assert_eq!( dst.len(), num * BLOCK_SIZE );
		
		let cmd = VirtioBlockReq {
			type_: VIRTIO_BLK_T_IN,
			ioprio: (255 - prio) as u32,
			sector: idx,
			};
		let mut status = 0u8;

		let rv = match self.requestq.send_buffers_blocking(&self.interface, &mut[
				Buffer::Read( ::kernel::lib::as_byte_slice(&cmd) ),
				Buffer::Write(dst),
				Buffer::Write( ::kernel::lib::as_byte_slice_mut(&mut status) )
				])
			{
			Ok(bytes) => Ok( bytes / BLOCK_SIZE ),
			Err( () ) => Err( storage::IoError::Unknown("VirtIO") ),
			};
		
		//log_debug!("read block {}", idx);
		//::kernel::logging::hex_dump("VirtIO block data", dst);
		
		Box::pin(async move { rv })
	}
	fn write<'a>(&'a self, prio: u8, idx: u64, num: usize, src: &'a [u8]) -> storage::AsyncIoResult<'a, usize>
	{
		assert_eq!( src.len(), num * BLOCK_SIZE );
		let cmd = VirtioBlockReq {
			type_: VIRTIO_BLK_T_OUT,
			ioprio: (255 - prio) as u32,
			sector: idx,
			};
		let mut status = 0u8;

		let rv = match self.requestq.send_buffers_blocking(&self.interface, &mut[
				Buffer::Read( ::kernel::lib::as_byte_slice(&cmd) ),
				Buffer::Read( src ),
				Buffer::Write( ::kernel::lib::as_byte_slice_mut(&mut status) )
				])
			{
			Ok(bytes) => Ok( bytes / BLOCK_SIZE ),
			Err( () ) => Err( storage::IoError::Unknown("VirtIO") ),
			};

		Box::pin(async move { rv })
	}
	
	fn wipe<'a>(&'a self, _blockidx: u64, _count: usize) -> storage::AsyncIoResult<'a,()>
	{
		// Do nothing, no support for TRIM
		Box::pin(async move { Ok(()) })
	}

}

