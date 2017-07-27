// "Tifflin" Kernel - SCSI Protocol Driver
// - By John Hodge (thePowersGang)
//
// Modules/storage_scsi/lib.rs
#![feature(linkage)]
#![no_std]
#[macro_use] extern crate kernel;
#[allow(unused_imports)]
use kernel::prelude::*;

use kernel::async;
use kernel::metadevs::storage;

pub mod proto;

pub trait ScsiInterface: Sync + Send + 'static
{
	fn name(&self) -> &str;
	fn send<'a>(&'a self, command: &[u8], data: &'a [u8]) -> storage::AsyncIoResult<'a,()>;
	fn recv<'a>(&'a self, command: &[u8], data: &'a mut [u8]) -> storage::AsyncIoResult<'a,()>;
}

#[derive(Debug)]
enum VolumeClass
{
	Unknown(u8),
	DirectAccessBlock,
	Sequential,
	CdDvd,
}

pub struct Volume<I: ScsiInterface>
{
	int: I,
	class: VolumeClass,
	// block size, number of blocks
	size: Option< (usize, u64) >,
}

impl<I: ScsiInterface> Volume<I>
{
	fn recv_cmd<'a>(int: &I, cmd: &[u8], data: &'a mut [u8]) -> Result<(), storage::IoError> {
		log_debug!("- cmd=[{:?}]", cmd);
		match cmd[0] & 0xE0
		{
		0x00 => assert_eq!(cmd.len(), 6),
		0x20 => assert_eq!(cmd.len(), 10),
		0x40 => assert_eq!(cmd.len(), 10),
		0xA0 => assert_eq!(cmd.len(), 12),
		0x80 => assert_eq!(cmd.len(), 16),
		_ => {},
		}

		let _size = {
			let mut v = int.recv(cmd, data);
			while !v.is_complete() {
				::kernel::async::wait_on_list(&mut [v.as_waiter()], None);
			}
			try!(v.get_result().unwrap())
			};
		Ok( () )
	}
	pub fn new_boxed(int: I) -> Result<Box<Self>,storage::IoError> {
		// 1. Request device type (INQUIRY)
		let (class, removable) = {
			let mut inq_data = proto::InquiryRsp::new();
			try!( Self::recv_cmd(&int, proto::Inquiry::new(inq_data.len() as u16).as_ref(), inq_data.as_mut()) );
			log_debug!("Type: {:#x}", inq_data.prehipheral_type());
			
			let class = match inq_data.prehipheral_type()
				{
				0x00 => VolumeClass::DirectAccessBlock,	// Direct access block (disk)
				0x01 => VolumeClass::Sequential,	// Sequential access device (tape)
				//0x02 => {},	// Printer
				//0x03 => {},	// Processor
				//0x04 => {},	// Write-once
				0x05 => VolumeClass::CdDvd,	// CD/DVD
				v @ _ => VolumeClass::Unknown(v),
				};
			let removable = inq_data.removable();
			
			(class, removable)
			};
		
		// 2. Check the size (and check for a disk too)
		let size = {
			let mut data = proto::ReadCapacity10Rsp::new();
			match Self::recv_cmd(&int, proto::ReadCapacity10::new().as_ref(), data.as_mut())
			{
			Ok(_) => {
				::kernel::logging::hex_dump("SCSI Volume size", data.as_ref());
				let blksz = data.block_length();
				let max = data.maxlba();
				Some( (blksz as usize, (max as u64 + 1)) )
				},
			Err(storage::IoError::NoMedium) if removable => {
				log_debug!("No medium");
				None
				},
			Err(e) => return Err(From::from(e)),
			}
			};
		log_log!("SCSI Volume {} - class={:?} size={:?}", int.name(), class, size);
		
		Ok(Box::new( Volume {
			int: int,
			class: class,
			size: size,
			} ))
	}
}

fn fits_in_bits(v: usize, bits: usize) -> bool {
	if bits >= ::core::mem::size_of::<usize>() * 8 {
		true
	}
	else {
		v < (1 << bits)
	}
}

impl<I: ScsiInterface> storage::PhysicalVolume for Volume<I>
{
	fn name(&self) -> &str { self.int.name() }
	fn blocksize(&self) -> usize { self.size.expect("Calling blocksize on no-media volume").0 }
	fn capacity(&self) -> Option<u64> { self.size.map(|x| x.1) }
	
	fn read<'a>(&'a self, _prio: u8, idx: u64, num: usize, dst: &'a mut [u8]) -> storage::AsyncIoResult<'a,usize>
	{
		// NOTE: Read6 commented out, as qemu's CD code doesn't support it
		let rv = /*if idx < (1<<24) && num < (1 << 8) {
				log_trace!("SCSI Read6");
				self.int.recv(proto::Read6::new(idx as u32, num as u8).as_ref(), dst)
			}
			else*/ if idx < (1<<32) && num < (1 << 16) {
				log_trace!("SCSI Read10");
				self.int.recv(proto::Read10::new(idx as u32, num as u16).as_ref(), dst)
			}
			else if /*idx < (1 << 64) &&*/ fits_in_bits(num, 32) {
				log_trace!("SCSI Read16");
				self.int.recv(proto::Read16::new(idx, num as u32).as_ref(), dst)
			}
			else {
				todo!("SCSI read out of range");
			};
		
		#[derive(Debug)]
		struct Wrapper<'a>(storage::AsyncIoResult<'a,()>, usize);
		impl<'a> ::kernel::async::Waiter for Wrapper<'a> {
			fn is_complete(&self) -> bool { self.0.is_complete() }
			fn get_waiter(&mut self) -> &mut ::kernel::async::PrimitiveWaiter { self.0.get_waiter() }
			fn complete(&mut self) -> bool { self.0.complete() }
		}
		impl<'a> ::kernel::async::ResultWaiter for Wrapper<'a> {
			type Result = Result<usize, ::kernel::metadevs::storage::IoError>;
			fn get_result(&mut self) -> Option<Self::Result> { self.0.get_result().map(|v| v.map(|_| self.1)) }
			fn as_waiter(&mut self) -> &mut ::kernel::async::Waiter { self.0.as_waiter() }
		}
		Box::new( Wrapper(rv, num) )

		//rv
	}
	fn write<'s>(&'s self, _prio: u8, idx: u64, num: usize, src: &'s [u8]) -> storage::AsyncIoResult<'s,usize> {
		match self.class
		{
		VolumeClass::CdDvd => Box::new(async::NullResultWaiter::new( || Err(storage::IoError::ReadOnly) )),
		VolumeClass::DirectAccessBlock => {
			todo!("Volume::write(idx={},num={},len={})", idx, num, src.len());
			},
		_ => Box::new(async::NullResultWaiter::new( || Err(storage::IoError::Unknown("TODO: Write support")) )),
		}
	}
	
	fn wipe<'a>(&'a self, _blockidx: u64, _count: usize) -> storage::AsyncIoResult<'a,()>
	{
		todo!("Volume::wipe");
	}
	
}

