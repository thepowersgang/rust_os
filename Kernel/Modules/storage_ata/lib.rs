//
//
//
#![feature(no_std,core)]
#![no_std]
#[macro_use] extern crate core;
#[macro_use] extern crate kernel;
use kernel::_common::*;
use kernel::sync::mutex::Mutex;
use kernel::lib::mem::Rc;
use kernel::memory::helpers::{DMABuffer, DescriptorPool};

use kernel::device_manager::IOBinding;
use kernel::async::{ReadHandle,WriteHandle,EventWait};

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


struct AtaVolume
{
	name: String,
	disk: u8,
	controller: Rc<AtaController>,
	
	size: u64,
}

struct AtaController
{
	int: Mutex<AtaControllerInt>,
}
struct AtaControllerInt
{
	ata_base: u16,
	dma_base: IOBinding,
	
	prdts: DescriptorPool<PRDTEnt>,	// A helper type for drivers, a pool of descriptors (capable of being taken/released arbitatrily)
}

#[repr(C)]
struct PRDTEnt
{
	addr: u32,
	bytes: u16,
	flags: u16,
}

impl ::kernel::metadevs::storage::PhysicalVolume for AtaVolume
{
	fn name(&self) -> &str { &*self.name }
	fn blocksize(&self) -> usize { SECTOR_SIZE }
	fn capacity(&self) -> u64 { self.size }
	
	fn read<'buf,'s>(&'s self, prio: u8, blockidx: u64, count: usize, dst: &'buf mut [u8]) -> Result<ReadHandle<'buf, 's>, ()>
	{
		assert_eq!( dst.len(), count * SECTOR_SIZE );
		let wh = try!( self.controller.do_dma(blockidx, count, dst as *mut [u8], self.disk, false));
		Ok( ReadHandle::new(dst, wh) )
	}
	fn write<'buf,'s>(&'s mut self, prio: u8, blockidx: u64, count: usize, dst: &'buf [u8]) -> Result<WriteHandle<'buf, 's>, ()>
	{
		assert_eq!( dst.len(), count * SECTOR_SIZE );
		let ctrlr = &self.controller;
		// Safe cast, as controller should not modify the buffer when write=true
		match ctrlr.do_dma(blockidx, count, dst as *const [u8] as *mut [u8], self.disk, true)
		{
		Err(e) => Err(e),
		Ok(v) => Ok( WriteHandle::new(dst, v) )
		}
	}
	
	fn wipe(&mut self, blockidx: u64, count: usize)
	{
		// Do nothing, no support for TRIM
	}
	
}

impl AtaControllerInt
{
	unsafe fn out_8(&self, ofs: u16, val: u8)
	{
		::kernel::arch::x86_io::outb(self.ata_base + ofs, val);
	}
	unsafe fn out_bm_32(&self, ofs: u16, val: u32)
	{
		unimplemented!();
	}
	unsafe fn out_bm_8(&self, ofs: u16, val: u8)
	{
		unimplemented!();
	}
}

impl AtaController
{
	
	fn wait_handle<F: FnOnce(&mut EventWait)> (&self, disk: u8, f: F) -> EventWait
	{
		unimplemented!();
	}
	
	fn do_dma<'s>(&'s self, blockidx: u64, count: usize, dst: *mut [u8], disk: u8, is_write: bool) -> Result<EventWait<'s>,()>
	{
		assert!(count < MAX_DMA_SECTORS);
		let int = self.int.lock();
		
		// Try to obtain a DMA context
		if let Some(mut prdt) = int.prdts.try_pop()
		{
			// Fill PRDT
			prdt.bytes = (count * SECTOR_SIZE) as u16;
			let dma_buffer = DMABuffer::new_contig( unsafe { &mut *dst }, 32 );
			prdt.addr = dma_buffer.phys() as u32;
			
			// Commence the IO and return a wait handle for the operation
			unsafe
			{
				// - Only use LBA48 if needed
				if blockidx >= (1 << 28)
				{
					int.out_8(6, 0x40 | (disk << 4));
					int.out_8(2, 0);	// Upper sector count (must be zero because of MAX_DMA_SECTORS)
					int.out_8(3, (blockidx >> 24) as u8);
					int.out_8(4, (blockidx >> 32) as u8);
					int.out_8(5, (blockidx >> 40) as u8);
				}
				else
				{
					int.out_8( 6, 0xE0 | (disk << 4) | ((blockidx >> 24) & 0x0F) as u8 );
				}
				int.out_8(2, count as u8);
				int.out_8(3, (blockidx >>  0) as u8);
				int.out_8(4, (blockidx >>  8) as u8);
				int.out_8(5, (blockidx >> 16) as u8);
				
				// - Set PRDT
				int.out_bm_32(4, prdt.phys() as u32);
				int.out_bm_8(0, 0x04);	// Reset IRQ
				
				if blockidx >= (1 << 48) {
					int.out_8(7, if is_write { HDD_DMA_W48 } else { HDD_DMA_R48 });
				}
				else {
					int.out_8(7, if is_write { HDD_DMA_W28 } else { HDD_DMA_R28 });
				}
				
				// Start IO
				int.out_8(0, 8 | 1);
			}
			drop(int);
			
			// Return the wait handle
			Ok( self.wait_handle(disk, |_| {
				drop(prdt);
				drop(dma_buffer);
				}) )
		}
		else
		{
			// If obtaining a context failed, put the request on the queue and return a wait handle for it
			unimplemented!();
		}
		
	}
}

