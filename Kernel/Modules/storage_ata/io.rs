//
//
//
//! ATA IO code, handling device multiplexing and IO operations
use kernel::_common::*;
use kernel::memory::helpers::{DMABuffer};
use kernel::async::Waiter;
use kernel::device_manager::IOBinding;

pub const SECTOR_SIZE: usize = 512;
const MAX_DMA_SECTORS: usize = 0x10000 / SECTOR_SIZE;	// Limited by byte count, 16-9 = 7 bits = 128 sectors

//const HDD_PIO_W28: u8 = 0x30,
//const HDD_PIO_R28: u8 = 0x20;
//const HDD_PIO_W48: u8 = 0x34;
//const HDD_PIO_R48: u8 = 0x24;
const HDD_IDENTIFY: u8 = 0xEC;

const HDD_DMA_R28: u8 = 0xC8;
const HDD_DMA_W28: u8 = 0xCA;
const HDD_DMA_R48: u8 = 0x25;
const HDD_DMA_W48: u8 = 0x35;

pub struct DmaController
{
	pub ata_controllers: [AtaController; 2],
	pub dma_base: IOBinding,
}
struct DmaRegBorrow<'a>
{
	dma_base: &'a IOBinding,
	is_sec: bool,
}
pub struct AtaController
{
	regs: ::kernel::async::Mutex<AtaRegs>,
	interrupt: AtaInterrupt,
}
struct AtaRegs
{
	ata_base: u16,
	sts_base: u16,
	prdts: ::kernel::memory::virt::ArrayHandle<PRDTEnt>,
}
struct AtaInterrupt
{
	handle: ::kernel::irqs::EventHandle,
	event: ::kernel::async::EventSource,
}

#[repr(C)]
struct PRDTEnt
{
	addr: u32,
	bytes: u16,
	flags: u16,
}

impl DmaController
{
	pub fn do_dma<'s>(&'s self, blockidx: u64, count: usize, dst: &'s [u8], disk: u8, is_write: bool) -> Result<Waiter<'s>,()>
	{
		assert!(disk < 4);
		assert!(count < MAX_DMA_SECTORS);
		assert_eq!(dst.len(), count * SECTOR_SIZE);
		
		let bus = (disk >> 1) & 1;
		let disk = disk & 1;
		
		// Try to obtain a DMA context
		Ok( self.ata_controllers[bus as usize].do_dma(blockidx, dst, disk, is_write, DmaRegBorrow::new(self, bus == 1) ) )
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

impl AtaRegs
{
	fn new(ata_base: u16, sts_port: u16) -> AtaRegs
	{
		AtaRegs {
			ata_base: ata_base, sts_base: sts_port,
			prdts: ::kernel::memory::virt::alloc_dma(32, 1, module_path!()).unwrap().into_array(),
		}
	}
	
	unsafe fn out_8(&mut self, ofs: u16, val: u8)
	{
		assert!(ofs < 8);
		::kernel::arch::x86_io::outb( self.ata_base + ofs, val);
	}
	
	unsafe fn in_8(&mut self, ofs: u16) -> u8
	{
		assert!(ofs < 8);
		::kernel::arch::x86_io::inb( self.ata_base + ofs )
	}
	unsafe fn in_16(&mut self, ofs: u16) -> u16
	{
		assert!(ofs < 8);
		::kernel::arch::x86_io::inw( self.ata_base + ofs )
	}
	// Safe - This port is a status port that does not affect the state
	fn in_sts(&self) -> u8
	{
		unsafe { ::kernel::arch::x86_io::inb( self.sts_base ) }
	}
	
	fn start_dma(&mut self, disk: u8, blockidx: u64, dma_buffer: &DMABuffer, is_write: bool, bm: DmaRegBorrow)
	{
		let count = dma_buffer.len() / SECTOR_SIZE;
		// Fill PRDT
		// TODO: Use a chain of PRDTs to support 32-bit scatter-gather
		self.prdts[0].bytes = dma_buffer.len() as u16;
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
	}
}

impl AtaController
{
	pub fn new(ata_base: u16, sts_port: u16, irq: u32) -> AtaController
	{
		AtaController {
			regs: ::kernel::async::Mutex::new( AtaRegs::new(ata_base, sts_port) ),
			interrupt: AtaInterrupt {
				handle: ::kernel::irqs::bind_interrupt_event(irq),
				event: ::kernel::async::EventSource::new(),
				},
			}
	}
	
	fn wait_handle<'a, F: FnOnce(&mut Waiter) + Send + 'a> (&'a self, f: F) -> Waiter<'a>
	{
		self.interrupt.event.wait_on(f)
	}
	
	fn do_dma<'a>(&'a self, blockidx: u64, dst: &'a [u8], disk: u8, is_write: bool, dma_regs: DmaRegBorrow) -> Waiter<'a>
	{
		let dma_buffer = DMABuffer::new_contig( unsafe { ::core::mem::transmute(dst) }, 32 );
		
		if let Some(mut buslock) = self.regs.try_lock()
		{
			buslock.start_dma( disk, blockidx, &dma_buffer, is_write, dma_regs );
			
			self.wait_handle( |_| { drop(dma_buffer); drop(buslock) } )
		}
		else
		{
			unimplemented!();
			// TODO: This following block of code has lifetime errors
			//// If obtaining a context failed, continue operation in a callback
			//self.regs.async_lock(|event_ref: &mut Waiter, mut buslock| {
			//	buslock.start_dma(disk, blockidx, &dma_buffer, is_write, dma_regs);
			//	*event_ref = self.wait_handle( |_| {
			//		drop(dma_buffer); drop(buslock)
			//		});
			//	})
		}
	}
	
	pub fn ata_identify<'a>(&'a self, disk: u8, data: &'a mut ::AtaIdentifyData) -> Waiter<'a>
	{
		// - Cast 'data' to a u16 slice
		let data: &mut [u16; 256] = unsafe { ::core::mem::transmute(data) };
		if let Some(mut buslock) = self.regs.try_lock()
		{
			log_debug!("ata_identify: (disk={}), base={:#x}", disk, buslock.ata_base);
			let status = unsafe {
				buslock.out_8(6, 0xA0 | (disk << 4) );
				buslock.out_8(2, 0);
				buslock.out_8(3, 0);
				buslock.out_8(4, 0);
				buslock.out_8(5, 0);
				buslock.out_8(7, HDD_IDENTIFY);
				buslock.in_8(7)
				};
			
			log_debug!("ata_identify: status = {:#02x}", status);
			if status == 0
			{
				log_debug!("Disk {} on {:#x} not present", disk, buslock.ata_base);
				// Drive does not exist, zero data and return a null wait
				*data = unsafe { ::core::mem::zeroed() };
				Waiter::new_none()
			}
			else
			{
				// Block until BSY clears
				// TODO: Timeout?
				while buslock.in_sts() & 0x80 != 0 { }
				
				// Return a poller
				Waiter::new_poll(move |e| match e
					{
					// Being called as a completion function
					Some(e) => {
						if buslock.in_sts() & 1 == 1 {
							let (f4, f5) = unsafe { (buslock.in_8(4), buslock.in_8(5)) };
							// Error, clear and return
							*data = unsafe { ::core::mem::zeroed() };
							if f4 == 0x14 && f5 == 0xEB {
								// Device is ATAPI
								log_debug!("ata_identify: Disk {:#x}/{} is ATAPI", buslock.ata_base, disk);
							}
							else {
								log_debug!("ata_identify: Disk {:#x}/{} errored (f4,f5 = {:#02x},{:#02x})", buslock.ata_base, disk, f4, f5);
							}
						}
						else {
							// Success, perform IO
							unsafe {
								for w in data.iter_mut() {
									*w = buslock.in_16(0);
								}
							}
							log_debug!("ata_identify: Disk {:#x}/{} IDENTIFY complete", buslock.ata_base, disk);
						}
						true
						},
					// Being called as a poll
					None => if buslock.in_sts() & 9 != 0 {
							// Done.
							true
						} else {
							false
						}
					} )
			}
		}
		else
		{
			panic!("Sending ATA identify while controller is in use");
		}
	}
}


