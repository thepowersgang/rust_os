
use kernel::prelude::*;
use kernel::metadevs::storage;
use kernel::async;
use interface::Interface;
use queue::{Queue,Buffer};


const VIRTIO_BLK_T_IN    	: u32 = 0;
const VIRTIO_BLK_T_OUT  	: u32 = 1;
const VIRTIO_BLK_T_SCSI_CMD	: u32 = 2;
const VIRTIO_BLK_T_SCSI_CMD_OUT	: u32 = 3;
const VIRTIO_BLK_T_FLUSH	: u32 = 4;
const VIRTIO_BLK_T_FLUSH_OUT: u32 = 5;
const VIRTIO_BLK_T_BARRIER	: u32 = 0x8000_0000;

pub struct BlockDevice
{
	pv_handle: storage::PhysicalVolumeReg,
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
		BlockDevice {
			pv_handle: storage::register_pv( Box::new(Volume{
				requestq: int.get_queue(0, 0).expect("Queue #0 'requestq' missing on virtio block device"),
				capacity: capacity,
				interface: int,
				}) ),
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

impl<I: Interface+Send+'static> storage::PhysicalVolume for Volume<I>
{
	fn name(&self) -> &str { "virtio" }
	fn blocksize(&self) -> usize { 512 }
	fn capacity(&self) -> Option<u64> { Some(self.capacity) }
	
	fn read<'a>(&'a self, prio: u8, idx: u64, num: usize, dst: &'a mut [u8]) -> storage::AsyncIoResult<'a,()>
	{
		assert_eq!( dst.len(), num * 512 );
		
		let cmd = VirtioBlockReq {
			type_: VIRTIO_BLK_T_IN,
			ioprio: (255 - prio) as u32,
			sector: idx,
			};
		let mut status = 0u8;

		let h = self.requestq.send_buffers(&self.interface, &mut[
			Buffer::Read( ::kernel::lib::as_byte_slice(&cmd) ),
			Buffer::Write(dst),
			Buffer::Write( ::core::slice::mut_ref_slice(&mut status) )
			]);
		h.wait_for_completion();

		Box::new(async::NullResultWaiter::new( || Ok( () ) ))
	}
	fn write<'a>(&'a self, _prio: u8, idx: u64, num: usize, src: &'a [u8]) -> storage::AsyncIoResult<'a,()>
	{
		todo!("");
	}
	
	fn wipe<'a>(&'a self, _blockidx: u64, _count: usize) -> storage::AsyncIoResult<'a,()>
	{
		// Do nothing, no support for TRIM
		Box::new(async::NullResultWaiter::new( || Ok( () ) ))
	}

}

