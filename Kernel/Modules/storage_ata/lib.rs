//
//
//
//! x86 ATA driver
#![feature(no_std,core)]
#![no_std]
#[macro_use] extern crate core;
#[macro_use] extern crate kernel;
use kernel::_common::*;
use kernel::lib::mem::Arc;

use kernel::device_manager;
use kernel::async::{ReadHandle,WriteHandle,EventWait};

module_define!{ATA, [DeviceManager, Storage], init}

mod drivers;
mod io;

struct AtaVolume
{
	name: String,
	disk: u8,
	controller: Arc<io::DmaController>,
	
	size: u64,
}

/// Initial controller handle, owns all volumes and the first controller handle
struct ControllerRoot
{
	_controller: Arc<io::DmaController>,
	volumes: Vec<AtaVolume>,
}

/// ATA "IDENTIFY" packet data
#[repr(C,packed)]
struct AtaIdentifyData
{
	flags: u16,
	_unused1: [u16; 9],
	serial_numer: [u8; 20],
	_unused2: [u16; 3],
	firmware_ver: [u8; 8],
	model_number: [u8; 40],
	/// NFI, TODO look up
	sect_per_int: u16,
	_unused3: u16,
	capabilities: [u16; 2],
	_unused4: [u16; 2],
	/// No idea
	valid_ext_data: u16,
	_unused5: [u16; 5],
	size_of_rw_multiple: u16,
	/// LBA 28 sector count (if zero, use 48)
	sector_count_28: u32,
	_unused6: [u16; 100-62],
	/// LBA 48 sector count
	sector_count_48: u64,
	_unused7: [u16; 256-104],
}
impl Default for AtaIdentifyData {
	fn default() -> AtaIdentifyData { unsafe { ::core::mem::zeroed() } }
}

fn init()
{
	drivers::register();
}

impl ::kernel::metadevs::storage::PhysicalVolume for AtaVolume
{
	fn name(&self) -> &str { &*self.name }
	fn blocksize(&self) -> usize { io::SECTOR_SIZE }
	fn capacity(&self) -> u64 { self.size }
	
	fn read<'s>(&'s self, _prio: u8, idx: u64, num: usize, dst: &'s mut [u8]) -> Result<ReadHandle<'s, 's>, ()>
	{
		assert_eq!( dst.len(), num * io::SECTOR_SIZE );
		let wh = try!( self.controller.do_dma(idx, num, dst, self.disk, false));
		Ok( ReadHandle::new(dst, wh) )
	}
	fn write<'s>(&'s self, _prio: u8, idx: u64, num: usize, src: &'s [u8]) -> Result<WriteHandle<'s, 's>, ()>
	{
		assert_eq!( src.len(), num * io::SECTOR_SIZE );
		let ctrlr = &self.controller;
		// Safe cast, as controller should not modify the buffer when write=true
		match ctrlr.do_dma(idx, num, src, self.disk, true)
		{
		Err(e) => Err(e),
		Ok(v) => Ok( WriteHandle::new(src, v) )
		}
	}
	
	fn wipe(&mut self, _blockidx: u64, _count: usize)
	{
		// Do nothing, no support for TRIM
	}
	
}

impl ControllerRoot
{
	fn new(ata_pri: u16, sts_pri: u16, irq_pri: u32,  ata_sec: u16, sts_sec: u16, irq_sec: u32,  bm: device_manager::IOBinding) -> ControllerRoot
	{
		let ctrlr_pri = io::AtaController::new(ata_pri, sts_pri, irq_pri);
		let ctrlr_sec = io::AtaController::new(ata_sec, sts_sec, irq_sec);
		
		// Send IDENTIFY to all disks
		for i in (0 .. 2)
		{
			let mut identify_pri: AtaIdentifyData = Default::default();
			let mut identify_sec: AtaIdentifyData = Default::default();
			
			let (pri_valid, sec_valid) = {
				let mut wh_pri = ctrlr_pri.ata_identify(i, &mut identify_pri);
				let mut wh_sec = ctrlr_sec.ata_identify(i, &mut identify_sec);
				//let mut wh_timer = ::kernel::async::Timer::new(2*1000);
				
				// Wait for both complete, and obtain results
				// - Loop while the timer hasn't fired, and at least one of the waiters is still waiting
				while /* !wh_timer.is_ready() && */ !(wh_pri.is_ready() && wh_sec.is_ready())
				{
					//::kernel::async::wait_on_list(&mut [&mut wh_pri, &mut wh_sec, &mut wh_timer]);
					::kernel::async::wait_on_list(&mut [&mut wh_pri, &mut wh_sec]);
				}
				
				(wh_pri.is_ready(), wh_sec.is_ready())
				};
			log_debug!("valid = {}, {}", pri_valid, sec_valid);
			if pri_valid {
				// Log
				log_log!("ATA{}: Size [LBA28 = {}, LBA48 = {}]", i*2, identify_pri.sector_count_28, identify_pri.sector_count_48);
			}
			if sec_valid {
				// Log
				log_log!("ATA{}: Size [LBA28 = {}, LBA48 = {}]", i*2+1, identify_sec.sector_count_28, identify_sec.sector_count_48);
			}
			
		}
		
		let dma_controller = Arc::new(io::DmaController {
			ata_controllers: [ ctrlr_pri, ctrlr_sec ],
			dma_base: bm,
			});
		
		ControllerRoot {
			_controller: dma_controller,
			volumes: Vec::new(),
			}
	}
}

impl device_manager::DriverInstance for ControllerRoot
{
	// Just a marker trait
}
