//
//
//
#![feature(no_std,core)]
#![no_std]
#[macro_use] extern crate core;
#[macro_use] extern crate kernel;
use kernel::_common::*;
use kernel::lib::mem::Arc;
use kernel::memory::helpers::{DMABuffer};

use kernel::device_manager;
use kernel::device_manager::IOBinding;
use kernel::async::{ReadHandle,WriteHandle,EventWait};

module_define!{ATA, [DeviceManager, Storage], init}

// Has a queue of IO operations, if a requested op cannot be started when requested, it's added to the queue

const SECTOR_SIZE: usize = 512;
const MAX_DMA_SECTORS: usize = 0x10000 / SECTOR_SIZE;	// Limited by byte count, 16-9 = 7 bits = 128 sectors

//const HDD_PIO_W28: u8 = 0x30,
//const HDD_PIO_R28: u8 = 0x20;
//const HDD_PIO_W48: u8 = 0x34;
//const HDD_PIO_R48: u8 = 0x24,
//const HDD_IDENTIFY: u8 = 0xEC

const HDD_DMA_R28: u8 = 0xC8;
const HDD_DMA_W28: u8 = 0xCA;
const HDD_DMA_R48: u8 = 0x25;
const HDD_DMA_W48: u8 = 0x35;

struct PciLegacyDriver;	// PCI Legacy ATA (BMDMA, all ports/IRQs legacy)
struct PciNativeDriver;	// PCI Native Mode ATA (all configured via PCI)

struct AtaVolume
{
	name: String,
	disk: u8,
	controller: Arc<DmaController>,
	
	size: u64,
}

/// Initial controller handle, owns all volumes and the first controller handle
struct ControllerRoot
{
	_controller: Arc<DmaController>,
	volumes: Vec<AtaVolume>,
}

struct DmaController
{
	ata_controllers: [::kernel::async::Mutex<AtaController>; 2],
	interrupts: [::kernel::async::EventSource; 2],
	dma_base: IOBinding,
}
struct DmaRegBorrow<'a>
{
	dma_base: &'a IOBinding,
	is_sec: bool,
}
struct AtaController
{
	ata_base: u16,
	sts_base: u16,
	prdts: ::kernel::memory::virt::ArrayHandle<PRDTEnt>,
}

#[allow(non_upper_case_globals)]
static s_pci_legacy_driver: PciLegacyDriver = PciLegacyDriver;
#[allow(non_upper_case_globals)]
static s_pci_native_driver: PciNativeDriver = PciNativeDriver;

#[repr(C)]
struct PRDTEnt
{
	addr: u32,
	bytes: u16,
	flags: u16,
}

fn init()
{
	device_manager::register_driver(&s_pci_legacy_driver);
	device_manager::register_driver(&s_pci_native_driver);
}

impl device_manager::Driver for PciLegacyDriver
{
	fn name(&self) -> &str {
		"ata-legacy"
	}
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &device_manager::BusDevice) -> u32
	{
		let classcode = bus_dev.get_attr("class");
		// [class] [subclass] [IF] [ver]
		if classcode & 0xFFFF0500 == 0x01010000 {
			1	// Handle as weakly as possible (vendor-provided drivers bind higher)
		}
		else {
			0
		}
	}
	fn bind(&self, bus_dev: &mut device_manager::BusDevice) -> Box<device_manager::DriverInstance+'static>
	{
		let bm_io = bus_dev.bind_io(4);
		Box::new( ControllerRoot::new(0x1F0, 0x3F6, 14,  0x170, 0x376, 15,  bm_io) )
	}
}

impl device_manager::Driver for PciNativeDriver
{
	fn name(&self) -> &str {
		"ata-native"
	}
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &device_manager::BusDevice) -> u32
	{
		let classcode = bus_dev.get_attr("class");
		// [class] [subclass] [IF] [ver]
		// IF ~= 0x05 means that both channels are in PCI native mode
		if classcode & 0xFFFF0500 == 0x01010500 {
			1	// Handle as weakly as possible (vendor-provided drivers bind higher)
		}
		else {
			0
		}
	}
	fn bind(&self, bus_dev: &mut device_manager::BusDevice) -> Box<device_manager::DriverInstance+'static>
	{
		let irq = bus_dev.get_irq(0);
		let io_pri = bus_dev.bind_io(0).io_base();
		let st_pri = bus_dev.bind_io(1).io_base() + 2;
		let io_sec = bus_dev.bind_io(2).io_base();
		let st_sec = bus_dev.bind_io(3).io_base() + 2;
		let bm_io = bus_dev.bind_io(4);
		Box::new( ControllerRoot::new(io_pri, st_pri, irq,  io_sec, st_sec, irq,  bm_io) )
	}
}

impl ::kernel::metadevs::storage::PhysicalVolume for AtaVolume
{
	fn name(&self) -> &str { &*self.name }
	fn blocksize(&self) -> usize { SECTOR_SIZE }
	fn capacity(&self) -> u64 { self.size }
	
	fn read<'s>(&'s self, _prio: u8, idx: u64, num: usize, dst: &'s mut [u8]) -> Result<ReadHandle<'s, 's>, ()>
	{
		assert_eq!( dst.len(), num * SECTOR_SIZE );
		let wh = try!( self.controller.do_dma(idx, num, dst, self.disk, false));
		Ok( ReadHandle::new(dst, wh) )
	}
	fn write<'s>(&'s self, _prio: u8, idx: u64, num: usize, src: &'s [u8]) -> Result<WriteHandle<'s, 's>, ()>
	{
		assert_eq!( src.len(), num * SECTOR_SIZE );
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
		// TODO: IRQs
		log_warning!("TODO: ControllerRoot::new - Handle IRQs ({} and {})", irq_pri, irq_sec);
		let ctrlr_pri = AtaController::new(ata_pri, sts_pri);
		let ctrlr_sec = AtaController::new(ata_sec, sts_sec);
		
		// Send IDENTIFY to all disks
		//for i in (0 .. 1)
		//{
		//	ctrlr_pri.send_identify(0);
		//	ctrlr_sec.send_identify(0);
		//	
		//	// Wait for both complete, and obtain results
		//}
		
		let dma_controller = Arc::new(DmaController {
			ata_controllers: [
				::kernel::async::Mutex::new(ctrlr_pri),
				::kernel::async::Mutex::new(ctrlr_sec),
				],
			interrupts: [
				::kernel::async::EventSource::new(),
				::kernel::async::EventSource::new(),
				],
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

impl DmaController
{
	fn wait_handle<'a, F: FnOnce(&mut EventWait) + Send + 'a> (&'a self, bus: u8, f: F) -> EventWait<'a>
	{
		self.interrupts[bus as usize].wait_on(f)
	}
	
	fn do_dma<'s>(&'s self, blockidx: u64, count: usize, dst: &'s [u8], disk: u8, is_write: bool) -> Result<EventWait<'s>,()>
	{
		assert!(disk < 4);
		assert!(count < MAX_DMA_SECTORS);
		assert_eq!(dst.len(), count * SECTOR_SIZE);
		
		let bus = (disk >> 1) & 1;
		let disk = disk & 1;
		
		// Try to obtain a DMA context
		// TODO: Block on the controller/disk
		if let Some(mut buslock) = self.ata_controllers[bus as usize].try_lock()
		{
			let dma_buffer = buslock.start_dma(
				disk, blockidx,
				dst, is_write,
				DmaRegBorrow::new(self, bus == 1)
				);
			// Return the wait handle
			Ok( self.wait_handle(bus, |_| {
				drop(dma_buffer);
				drop(buslock);
				}) )
		}
		else
		{
			// If obtaining a context failed, put the request on the queue and return a wait handle for it
			//Ok( self.ata_controllers[bus].async_lock() )
			unimplemented!();
		}
		
	}
}

impl<'a> DmaRegBorrow<'a>
{
	fn new(dm: &DmaController, is_secondary: bool) -> DmaRegBorrow
	{
		DmaRegBorrow {
			dma_base: &dm.dma_base,
			is_sec: is_secondary,
		}
	}
	
	unsafe fn out_32(&self, ofs: u16, val: u32)
	{
		assert!(ofs < 8);
		self.dma_base.write_32( if self.is_sec { 8 } else { 0 } + ofs as usize, val );
		unimplemented!();
	}
	unsafe fn out_8(&self, ofs: u16, val: u8)
	{
		assert!(ofs < 8);
		self.dma_base.write_8( if self.is_sec { 8 } else { 0 } + ofs as usize, val );
	}
	
}

impl AtaController
{
	fn new(ata_base: u16, sts_port: u16) -> AtaController
	{
		AtaController {
			ata_base: ata_base, sts_base: sts_port,
			prdts: ::kernel::memory::virt::alloc_dma(32, 1, module_path!()).unwrap().into_array(),
			}
	}
	
	unsafe fn out_8(&self, ofs: u16, val: u8)
	{
		::kernel::arch::x86_io::outb( self.ata_base + ofs, val);
	}
	
	fn start_dma<'a>(&mut self, disk: u8, blockidx: u64, buf: &'a [u8], is_write: bool, bm: DmaRegBorrow) -> DMABuffer<'a>
	{
		let dma_buffer = DMABuffer::new_contig( unsafe { ::core::mem::transmute(buf) }, 32 );
		
		let count = buf.len() / SECTOR_SIZE;
		// Fill PRDT
		// TODO: Use a chain of PRDTs to support 32-bit scatter-gather
		self.prdts[0].bytes = buf.len() as u16;
		self.prdts[0].addr = dma_buffer.phys() as u32;
		
		// Commence the IO and return a wait handle for the operation
		unsafe
		{
			// - Only use LBA48 if needed
			if blockidx >= (1 << 28)
			{
				self.out_8(6, 0x40 | (disk << 4));
				self.out_8(2, 0);	// Upper sector count (must be zero because of MAX_DMA_SECTORS)
				self.out_8(3, (blockidx >> 24) as u8);
				self.out_8(4, (blockidx >> 32) as u8);
				self.out_8(5, (blockidx >> 40) as u8);
			}
			else
			{
				self.out_8(6, 0xE0 | (disk << 4) | ((blockidx >> 24) & 0x0F) as u8);
			}
			self.out_8(2, count as u8);
			self.out_8(3, (blockidx >>  0) as u8);
			self.out_8(4, (blockidx >>  8) as u8);
			self.out_8(5, (blockidx >> 16) as u8);
			
			// - Set PRDT
			bm.out_32(4, ::kernel::memory::virt::get_phys(&self.prdts[0]) as u32);
			bm.out_8(0, 0x04);	// Reset IRQ
			
			self.out_8(7,
				if blockidx >= (1 << 48) {
					if is_write { HDD_DMA_W48 } else { HDD_DMA_R48 }	// LBA 48
				} else {
					if is_write { HDD_DMA_W28 } else { HDD_DMA_R28 }	// LBA 28
				});
			
			// Start IO
			bm.out_8(0, if is_write { 0 } else { 8 } | 1);
		}
		dma_buffer
	}
}

