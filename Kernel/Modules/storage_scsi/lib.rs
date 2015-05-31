// "Tifflin" Kernel - SCSI Protocol Driver
// - By John Hodge (thePowersGang)
//
// Modules/storage_scsi/lib.rs
#![feature(no_std,core,linkage)]
#![feature(associated_consts)]
#![no_std]
#[macro_use] extern crate core;
#[macro_use] extern crate kernel;
use kernel::prelude::*;

use kernel::metadevs::storage;
use kernel::async::{self, AsyncResult};

pub mod proto;

pub trait ScsiInterface: Sync + Send + 'static
{
	fn name(&self) -> &str;
	fn send<'a>(&'a self, command: &'a [u8], data: &'a [u8]) -> AsyncResult<'a,usize,storage::IoError>;
	fn recv<'a>(&'a self, command: &'a [u8], data: &'a mut [u8]) -> AsyncResult<'a,usize,storage::IoError>;
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
			Ok(d) => {
				::kernel::logging::hex_dump("SCSI Volume size", data.as_ref());
				let blksz = data.block_length();
				let max = data.maxlba();
				Some( (blksz as usize, (max as u64 + 1)) )
				},
			Err(storage::IoError::NoMedium) if removable => {
				log_debug!("No medium");
				None
				},
			Err(e) => todo!("handle error while initialising SCSI volume: {:?}", e),
			}
			};
		log_debug!("SCSI Volume {:?} size={:?}", class, size);
		
		Ok(Box::new( Volume {
			int: int,
			class: class,
			size: size,
			} ))
	}
}

impl<I: ScsiInterface> storage::PhysicalVolume for Volume<I>
{
	fn name(&self) -> &str { self.int.name() }
	fn blocksize(&self) -> usize { self.size.expect("Calling blocksize on no-media volume").0 }
	fn capacity(&self) -> Option<u64> { self.size.map(|x| x.1) }
	
	fn read<'a>(&'a self, _prio: u8, idx: u64, num: usize, dst: &'a mut [u8]) -> Result<Box<async::Waiter+'a>, storage::IoError>
	{
		todo!("Volume::read");
	}
	fn write<'s>(&'s self, _prio: u8, idx: u64, num: usize, src: &'s [u8]) -> Result<Box<async::Waiter+'s>, storage::IoError>
	{
		todo!("Volume::write");
	}
	
	fn wipe(&mut self, _blockidx: u64, _count: usize) -> Result<(),storage::IoError>
	{
		todo!("Volume::wipe");
	}
	
}

