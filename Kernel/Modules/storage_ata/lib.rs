// "Tifflin" Kernel - ATA Driver
// - By John Hodge (thePowersGang)
//
// Modules/storage_ata/lib.rs
//! x86 ATA driver
#![feature(no_std,core)]
#![no_std]
#[macro_use] extern crate core;
#[macro_use] extern crate kernel;
use kernel::_common::*;
use kernel::lib::mem::Arc;

use kernel::device_manager;
use kernel::metadevs::storage;
use kernel::async;

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
	_volumes: Vec<storage::PhysicalVolumeReg>,
}

enum AtaClass
{
	Invalid,	// No valid response (timeout)
	None,	// No disk
	Unknown(u8,u8),	// Unknown type, values are regs 4 and 5
	Native,	// A standard ATA disk
	ATAPI,
}
impl Default for AtaClass { fn default() -> AtaClass { AtaClass::Invalid } }

/// ATA "IDENTIFY" packet data
#[repr(C,packed)]
struct AtaIdentifyData
{
	flags: u16,
	_unused1: [u16; 9],
	serial_number: [u8; 20],
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
impl ::core::fmt::Debug for AtaIdentifyData {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		try!(write!(f, "AtaIdentifyData {{"));
		try!(write!(f, " flags: {:#x}", self.flags));
		try!(write!(f, " serial_number: {:?}", ::kernel::lib::RawString(&self.serial_number)));
		try!(write!(f, " firmware_ver: {:?}", ::kernel::lib::RawString(&self.firmware_ver)));
		try!(write!(f, " model_number: {:?}", ::kernel::lib::RawString(&self.model_number)));
		try!(write!(f, " sect_per_int: {}", self.sect_per_int));
		try!(write!(f, " capabilities: [{:#x},{:#x}]", self.capabilities[0], self.capabilities[1]));
		try!(write!(f, " valid_ext_data: {}", self.valid_ext_data));
		try!(write!(f, " size_of_rw_multiple: {}", self.size_of_rw_multiple));
		try!(write!(f, " sector_count_28: {:#x}", self.sector_count_28));
		try!(write!(f, " sector_count_48: {:#x}", self.sector_count_48));
		try!(write!(f, "}}"));
		Ok( () )
	}
}

fn init()
{
	drivers::register();
}

impl AtaVolume
{
	fn new_boxed(dma_controller: Arc<io::DmaController>, disk: u8, sectors: u64) -> Box<AtaVolume>
	{
		Box::new( AtaVolume {
			name: format!("{}-{}", dma_controller.name, disk),
			disk: disk,
			controller: dma_controller,
			size: sectors,
			} )
	}
}

impl ::kernel::metadevs::storage::PhysicalVolume for AtaVolume
{
	fn name(&self) -> &str { &*self.name }
	fn blocksize(&self) -> usize { io::SECTOR_SIZE }
	fn capacity(&self) -> u64 { self.size }
	
	fn read(&self, _prio: u8, idx: u64, num: usize, dst: &mut [u8]) -> Result<Box<async::Waiter>, ()>
	{
		assert_eq!( dst.len(), num * io::SECTOR_SIZE );
		Ok( try!( self.controller.do_dma(idx, num, dst, self.disk, false)) )
	}
	fn write<'s>(&'s self, _prio: u8, idx: u64, num: usize, src: &'s [u8]) -> Result<Box<async::Waiter+'s>, ()>
	{
		assert_eq!( src.len(), num * io::SECTOR_SIZE );
		let ctrlr = &self.controller;
		// Safe cast, as controller should not modify the buffer when write=true
		Ok( try!(ctrlr.do_dma(idx, num, src, self.disk, true)) )
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
		
		let dma_controller = Arc::new(io::DmaController {
			name: format!("ATA[{:#x},{:#x}]", ata_pri, ata_sec),
			ata_controllers: [
				io::AtaController::new(ata_pri, sts_pri, irq_pri),
				io::AtaController::new(ata_sec, sts_sec, irq_sec),
				],
			dma_base: bm,
			});
		let mut volumes = Vec::new();
		
		// Send IDENTIFY to all disks
		for i in (0 .. 2)
		{
			let ctrlr_pri = &dma_controller.ata_controllers[0];
			let ctrlr_sec = &dma_controller.ata_controllers[1];
			
			// Create output data (defaulted, but should be written by the output function)
			let (mut identify_pri, mut type_pri) = Default::default();
			let (mut identify_sec, mut type_sec) = Default::default();
			
			// Perform IDENTIFY requests, both controllers in pararllel
			// TODO: Include a timeout to prevent a misbehaving controller from halting the system.
			{
				use kernel::async::Waiter;
				
				let mut wh_pri = ctrlr_pri.ata_identify(i, &mut identify_pri, &mut type_pri);
				let mut wh_sec = ctrlr_sec.ata_identify(i, &mut identify_sec, &mut type_sec);
				//let mut wh_timer = ::kernel::async::Timer::new(2*1000);
				
				// Wait for both complete, and obtain results
				// - Loop while the timer hasn't fired, and at least one of the waiters is still waiting
				while /* !wh_timer.is_complete() && */ !(wh_pri.is_complete() && wh_sec.is_complete())
				{
					//::kernel::async::wait_on_list(&mut [&mut wh_pri, &mut wh_sec, &mut wh_timer]);
					::kernel::async::wait_on_list(&mut [&mut wh_pri, &mut wh_sec], None);
				}
			}
			
			// (ugly) Handle the relevant disk types, creating devices
			for &(disk, ref class, ref ident) in [(i*2, type_pri, identify_pri), (i*2+1, type_sec, identify_sec)].iter()
			{
				match *class
				{
				AtaClass::Invalid => {
					log_log!("ATA{}: Timeout", disk);
					},
				AtaClass::None => {
					log_log!("ATA{}: No disk", disk);
					},
				AtaClass::Native => {
					let sectors = if ident.sector_count_48 == 0 { ident.sector_count_28 as u64 } else { ident.sector_count_48 };
					log_log!("ATA{}: Hard Disk, {} sectors, {}", disk, sectors, storage::SizePrinter(sectors * io::SECTOR_SIZE as u64));
					volumes.push( storage::register_pv( AtaVolume::new_boxed(dma_controller.clone(), disk, sectors) ) );
					},
				AtaClass::ATAPI => {
					log_log!("ATA{}: ATAPI", disk);
					// TODO: Support ATAPI devices with a different class
					},
				AtaClass::Unknown(r4, r5) => {
					log_warning!("ATA{}: Unknown type response ({:#x}, {:#x})", disk, r4, r5);
					},
				}
			}
		}
		
		// Return a controller handle, holding on to all handles
		ControllerRoot { _controller: dma_controller, _volumes: volumes, }
	}
	
	//fn handle_volume(volumes: &mut Vec<storage::PhysicalVolumeReg>, dma_controller: &Arc<DmaController>, disk: u8, class: AtaClass, ident: AtaIdentifyData)
}

impl device_manager::DriverInstance for ControllerRoot
{
	// Just a marker trait
}
