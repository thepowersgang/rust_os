//
//
//
//! ATA IO code, handling device multiplexing and IO operations
use kernel::prelude::*;
use kernel::memory::helpers::{DMABuffer};
use kernel::async;
use kernel::metadevs::storage;
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
	pub name: String,
	pub ata_controllers: [AtaController; 2],
	pub dma_base: IOBinding,
}
struct DmaRegBorrow<'a>
{
	dma_base: &'a IOBinding,
	is_sec: bool,
}
struct DmaStatusVal(u8);
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
struct AtaStatusVal(u8);
struct AtaInterrupt
{
	handle: ::kernel::irqs::EventHandle,
}

#[repr(C)]
struct PRDTEnt
{
	addr: u32,
	bytes: u16,
	flags: u16,
}
impl_fmt!{
	Debug(self,f) for PRDTEnt {
		write!(f, "PRDTEnt {{ {:#X}+{}b, {:#x} }}", self.addr, self.bytes, self.flags)
	}
}

impl DmaController
{
	pub fn do_dma_rd<'a>(&'a self, blockidx: u64, count: usize, dst: &'a mut [u8], disk: u8) -> Result<Box<async::Waiter+'a>,storage::IoError> {
		self.do_dma(blockidx, count, DMABuffer::new_contig_mut(dst, 32), disk, false)
	}
	pub fn do_dma_wr<'a>(&'a self, blockidx: u64, count: usize, dst: &'a [u8], disk: u8) -> Result<Box<async::Waiter+'a>,storage::IoError> {
		self.do_dma(blockidx, count, DMABuffer::new_contig(dst, 32), disk, true)
	}
	fn do_dma<'a>(&'a self, blockidx: u64, count: usize, dst: DMABuffer<'a>, disk: u8, is_write: bool) -> Result<Box<async::Waiter+'a>,storage::IoError>
	{
		assert!(disk < 4);
		assert!(count < MAX_DMA_SECTORS);
		assert_eq!(dst.len(), count * SECTOR_SIZE);
		
		let bus = (disk >> 1) & 1;
		let disk = disk & 1;
		
		// Try to obtain a DMA context
		let ctrlr = &self.ata_controllers[bus as usize];
		let bm_regs = self.borrow_regs(bus == 1);
		
		let ub = ctrlr.do_dma(blockidx, dst, disk, is_write, bm_regs);
		let b = Box::new(ub);
		Ok( b )
	}
	fn borrow_regs(&self, is_secondary: bool) -> DmaRegBorrow {
		DmaRegBorrow {
			dma_base: &self.dma_base,
			is_sec: is_secondary,
		}
	}
}

impl<'a> DmaRegBorrow<'a>
{
	unsafe fn out_32(&self, ofs: u16, val: u32)
	{
		assert!(ofs < 8);
		self.dma_base.write_32( if self.is_sec { 8 } else { 0 } + ofs as usize, val );
	}
	unsafe fn out_8(&self, ofs: u16, val: u8)
	{
		assert!(ofs < 8);
		self.dma_base.write_8( if self.is_sec { 8 } else { 0 } + ofs as usize, val );
	}
	unsafe fn in_8(&self, ofs: u16) -> u8
	{
		assert!(ofs < 8);
		self.dma_base.read_8( if self.is_sec { 8 } else { 0 } + ofs as usize )
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
	
	fn start_dma(&mut self, disk: u8, blockidx: u64, dma_buffer: &DMABuffer, is_write: bool, bm: &DmaRegBorrow)
	{
		log_debug!("start_dma(disk={},blockidx={},is_write={},dma_buffer={{len={}}})",
			disk, blockidx, is_write, dma_buffer.len());
		let count = dma_buffer.len() / SECTOR_SIZE;
		// Fill PRDT
		// TODO: Use a chain of PRDTs to support 32-bit scatter-gather
		self.prdts[0].bytes = dma_buffer.len() as u16;
		self.prdts[0].addr = dma_buffer.phys() as u32;
		self.prdts[0].flags = 0x8000;
		
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

enum WaitState<'dev>
{
	Acquire(async::mutex::Waiter<'dev,AtaRegs>),
	IoActive(async::mutex::HeldMutex<'dev,AtaRegs>, async::event::Waiter<'dev>),
	Done,
}
struct AtaWaiter<'dev,'buf>
{
	dev: &'dev AtaController,
	disk: u8,
	blockidx: u64,
	is_write: bool,
	dma_regs: DmaRegBorrow<'dev>,
	dma_buffer: DMABuffer<'buf>,
	state: WaitState<'dev>,
}

impl<'a,'b> async::Waiter for AtaWaiter<'a,'b>
{
	fn is_complete(&self) -> bool {
		if let WaitState::Done = self.state { true } else { false }
	}
	
	fn get_waiter(&mut self) -> &mut async::PrimitiveWaiter
	{
		match self.state
		{
		// Initial state: Acquire the register lock
		WaitState::Acquire(ref mut waiter) => waiter,
		// Final state: Start IO and wait for it to complete
		WaitState::IoActive(_, ref mut waiter) => waiter,
		//
		WaitState::Done => unreachable!(),
		}
	}
	
	fn complete(&mut self) -> bool
	{
		// Update state if the match returns
		self.state = match self.state
			{
			// If the Acquire wait completed, switch to IoActive state
			WaitState::Acquire(ref mut waiter) => {
				let mut lh = waiter.take_lock();
				lh.start_dma( self.disk, self.blockidx, &self.dma_buffer, self.is_write, &self.dma_regs );
				WaitState::IoActive(lh, self.dev.interrupt.handle.get_event().wait())
				},
			// And if IoActive completes, we're complete
			WaitState::IoActive(ref mut lh, ref _waiter) => {
				// SAFE: Holding the register lock
				unsafe {
					self.dma_regs.out_8(0, 0);	// Stop transfer
					let ata_status = lh.in_8(7);
					log_trace!("BM Status = {:?}, ATA Status = {:?}",
						DmaStatusVal(self.dma_regs.in_8(2)), AtaStatusVal(ata_status)
						);
				}
				WaitState::Done
				},
			//
			WaitState::Done => unreachable!(),
			};
		
		self.is_complete()
	}
}
impl<'a,'b> ::core::fmt::Debug for AtaWaiter<'a,'b> {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		try!( write!(f, "AtaWaiter") );
		match self.state
		{
		WaitState::Acquire(..) => write!(f, "(Acquire)"),
		WaitState::IoActive(..) => write!(f, "(IoActive)"),
		WaitState::Done => write!(f, "(Done)"),
		}
	}
}

impl AtaController
{
	pub fn new(ata_base: u16, sts_port: u16, irq: u32) -> AtaController
	{
		AtaController {
			regs: async::Mutex::new( AtaRegs::new(ata_base, sts_port) ),
			interrupt: AtaInterrupt {
				handle: ::kernel::irqs::bind_event(irq),
				},
			}
	}
	
	fn do_dma<'a,'b>(&'a self, blockidx: u64, dst: DMABuffer<'b>, disk: u8, is_write: bool, dma_regs: DmaRegBorrow<'a>) -> AtaWaiter<'a,'b>
	{
		AtaWaiter {
			dev: self,
			disk: disk,
			blockidx: blockidx,
			is_write: is_write,
			dma_regs: dma_regs,
			dma_buffer: dst,
			state: WaitState::Acquire( self.regs.async_lock() ),
		}
	}
	
	/// Request an ATA IDENTIFY packet from the device
	pub fn ata_identify<'a>(&'a self, disk: u8, data: &'a mut ::AtaIdentifyData, class: &'a mut ::AtaClass) -> async::poll::Waiter<'a>
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
				*class = ::AtaClass::None;
				*data = unsafe { ::core::mem::zeroed() };
				async::poll::Waiter::null()
			}
			else
			{
				// Block until BSY clears
				// TODO: Timeout?
				while buslock.in_sts() & 0x80 != 0 { }
				
				// Return a poller
				async::poll::Waiter::new(move |e| match e
					{
					// Being called as a completion function
					Some(_event_ptr) => {
						if buslock.in_sts() & 1 == 1 {
							let (f4, f5) = unsafe { (buslock.in_8(4), buslock.in_8(5)) };
							// Error, clear and return
							*data = unsafe { ::core::mem::zeroed() };
							if f4 == 0x14 && f5 == 0xEB {
								// Device is ATAPI
								log_debug!("ata_identify: Disk {:#x}/{} is ATAPI", buslock.ata_base, disk);
								*class = ::AtaClass::ATAPI;
							}
							else {
								log_debug!("ata_identify: Disk {:#x}/{} errored (f4,f5 = {:#02x},{:#02x})", buslock.ata_base, disk, f4, f5);
								*class = ::AtaClass::Unknown(f4, f5);
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
							*class = ::AtaClass::Native;
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

impl_fmt! {
	Debug(self,f) for DmaStatusVal {{
		try!(write!(f, "({:#x}", self.0));
		if self.0 & (1<<0) != 0 { try!(write!(f, " DMAing")); }
		if self.0 & (1<<1) != 0 { try!(write!(f, " Fail")); }
		if self.0 & (1<<2) != 0 { try!(write!(f, " IRQ")); }
		if self.0 & (1<<5) != 0 { try!(write!(f, " MasterS")); }
		if self.0 & (1<<6) != 0 { try!(write!(f, " SlaveS")); }
		if self.0 & (1<<7) != 0 { try!(write!(f, " SO")); }
		write!(f, ")")
	}}
	Debug(self,f) for AtaStatusVal {{
		try!(write!(f, "({:#x}", self.0));
		if self.0 & (1<<0) != 0 { try!(write!(f, " ERR")); }
		if self.0 & (1<<3) != 0 { try!(write!(f, " DRQ")); }
		if self.0 & (1<<4) != 0 { try!(write!(f, " SRV")); }
		if self.0 & (1<<5) != 0 { try!(write!(f, " DF" )); }
		if self.0 & (1<<6) != 0 { try!(write!(f, " RDY")); }
		if self.0 & (1<<7) != 0 { try!(write!(f, " BSY")); }
		write!(f, ")")
	}}
}

