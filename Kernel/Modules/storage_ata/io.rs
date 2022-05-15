//
//
//
//! ATA IO code, handling device multiplexing and IO operations
use ::kernel::prelude::*;
use ::kernel::memory::helpers::{DMABuffer};
use ::kernel::metadevs::storage;
use ::kernel::device_manager::IOBinding;

pub const SECTOR_SIZE: usize = 512;
//const MAX_DMA_SECTORS: usize = 0x2_0000 / SECTOR_SIZE;	// Limited by sector count (and PRDT entries)
const MAX_DMA_SECTORS: usize = 0x1F_F000 / SECTOR_SIZE;	// Limited by sector count (and PRDT entries)
// 512 PDRT entries, assume maximum fragmentation = 512 * 4K max = 2^21 = 2MB per transfer

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
	regs: ::kernel::futures::Mutex<AtaRegs>,
	interrupt: AtaInterrupt,
}
struct AtaRegs
{
	ata_base: u16,
	sts_base: u16,
	prdts: ::kernel::memory::virt::ArrayHandle<PRDTEnt>,
}
struct AtaStatusVal(u8);
struct AtaErrorVal(u8);
struct AtapiErrorVal(u8);
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
	fn borrow_regs(&self, is_secondary: bool) -> DmaRegBorrow {
		DmaRegBorrow {
			dma_base: &self.dma_base,
			is_sec: is_secondary,
		}
	}

	/// Read ATA DMA
	pub fn do_dma_rd<'a>(&'a self, blockidx: u64, count: usize, dst: &'a mut [u8], disk: u8) -> storage::AsyncIoResult<'a,usize> {
		assert_eq!(dst.len(), count * SECTOR_SIZE);
		let dst = if count > MAX_DMA_SECTORS { &mut dst[.. MAX_DMA_SECTORS * SECTOR_SIZE] } else { dst };
		self.do_dma(blockidx, DMABuffer::new_mut(dst, 32), disk, false)
	}
	/// Write ATA DMA
	pub fn do_dma_wr<'a>(&'a self, blockidx: u64, count: usize, dst: &'a [u8], disk: u8) -> storage::AsyncIoResult<'a,usize> {
		assert_eq!(dst.len(), count * SECTOR_SIZE);
		let dst = if count > MAX_DMA_SECTORS { &dst[.. MAX_DMA_SECTORS * SECTOR_SIZE] } else { dst };
		self.do_dma(blockidx, DMABuffer::new(dst, 32), disk, true)
	}
	fn do_dma<'a>(&'a self, blockidx: u64, dst: DMABuffer<'a>, disk: u8, is_write: bool) -> storage::AsyncIoResult<'a,usize>
	{
		log_trace!("do_dma(blockidx={}, dst={:?}, disk={})", blockidx, dst, disk);
		assert!(disk < 4);
		assert!(dst.len() <= MAX_DMA_SECTORS * SECTOR_SIZE);
		
		let bus = (disk >> 1) & 1;
		let disk = disk & 1;
		
		// Try to obtain a DMA context
		let ctrlr = &self.ata_controllers[bus as usize];
		let bm_regs = self.borrow_regs(bus == 1);
		
		let ub = ctrlr.do_dma(blockidx, dst, disk, is_write, bm_regs);
		Box::pin(ub)
	}
	
	pub fn do_atapi_rd<'a>(&'a self, disk: u8, cmd: &[u8], dst: &'a mut [u8]) -> storage::AsyncIoResult<'a,()> {
		self.do_atapi(disk, cmd, DMABuffer::new_mut(dst, 32), false)
	}
	pub fn do_atapi_wr<'a>(&'a self, disk: u8, cmd: &[u8], dst: &'a [u8]) -> storage::AsyncIoResult<'a,()> {
		self.do_atapi(disk, cmd, DMABuffer::new(dst, 32), true)
	}
	fn do_atapi<'a>(&'a self, disk: u8, cmd: &[u8], dst: DMABuffer<'a>, is_write: bool) -> storage::AsyncIoResult<'a,()>
	{
		assert!(disk < 4);
		
		let bus = (disk >> 1) & 1;
		let disk = disk & 1;
		
		// Try to obtain a DMA context
		let ctrlr = &self.ata_controllers[bus as usize];
		let bm_regs = self.borrow_regs(bus == 1);
		
		let ub = ctrlr.do_atapi(disk, bm_regs, cmd, dst, is_write);
		Box::pin(ub)
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
	
	#[allow(dead_code)]
	unsafe fn dump(&mut self) {
		log_trace!("[0:1] {:#02x} {:#02x}", self.in_8(0), self.in_8(1));
		log_trace!("[2:3] {:#02x} {:#02x}", self.in_8(2), self.in_8(3));
		log_trace!("[4:5] {:#02x} {:#02x}", self.in_8(4), self.in_8(5));
		log_trace!("[6:7] {:#02x} {:#02x}", self.in_8(6), self.in_8(7));
		for e in &*self.prdts {
			log_trace!("{:#x}+{:#x} {:#x}", e.addr, e.bytes, e.flags);
			if e.flags & 0x8000 != 0 {
				break;
			}
		}
	}
	
	unsafe fn out_8(&mut self, ofs: u16, val: u8)
	{
		assert!(ofs < 8);
		::kernel::arch::x86_io::outb( self.ata_base + ofs, val);
	}
	unsafe fn out_16(&mut self, ofs: u16, val: u16)
	{
		assert!(ofs < 8);
		::kernel::arch::x86_io::outw( self.ata_base + ofs, val);
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
	fn read_sector(&mut self, data: &mut [u16])
	{
		// SAFE: Called with &mut, no race. Only reads data port
		unsafe {
			for w in data.iter_mut() {
				*w = self.in_16(0);
			}
		}
	}
	fn in_sts(&self) -> u8
	{
		// SAFE: Status port has no side-effects
		unsafe { ::kernel::arch::x86_io::inb( self.sts_base ) }
	}
	
	fn last_result(&mut self, is_atapi: bool) -> Result<(),storage::IoError> {
		let sts = self.in_sts();
		if sts & AtaStatusVal::ERR != 0 {
			// SAFE: Locked
			let err = unsafe { self.in_8(1) };
			Err(if is_atapi
				{
					log_trace!("err = {:?}", AtapiErrorVal(err));
					match AtapiErrorVal(err).sense_key()
					{
					AtapiErrorVal::NOT_READY => storage::IoError::NoMedium,
					AtapiErrorVal::ILLEGAL_REQUEST => storage::IoError::InvalidParameter,
					_ => storage::IoError::Unknown("ATAPI Error code"),
					}
				}
				else
				{
					log_trace!("err = {:?}", AtaErrorVal(err));
					storage::IoError::Unknown("ata")
				})
		}
		else if sts & AtaStatusVal::DF != 0 {
			Err(storage::IoError::Unknown("Drive fault"))
		}
		else if sts & AtaStatusVal::RDY == 0 {
			Err(storage::IoError::Timeout)
		}
		else {
			Ok( () )
		}
	}
	
	fn fill_prdt(&mut self, dma_buffer: &DMABuffer)// -> Result<(), usize>
	{
		// TODO: Handle running out of entries.
		// - Should ensure that transfer is aligned to 512 bytes (or to sector size)

		// Fill PRDT
		let mut count = 0;
		{
			let mut prdt_ents = self.prdts.iter_mut();
			for region in dma_buffer.phys_ranges()
			{
				let mut paddr = region.0;
				let mut bytes = region.1;
				while bytes > 0
				{
					let prd_ent = match prdt_ents.next()
						{
						Some(v) => v,
						None => todo!("Ran out of PRDT ents, return early with truncated read"),
						};
					let ent_bytes = if bytes >= 0x1_0000 { 0xFFFF } else { bytes };

					prd_ent.bytes = ent_bytes as u16;
					prd_ent.addr = ::core::convert::TryFrom::try_from(paddr).expect("Physical address out of range for 32-bit PRDT");
					prd_ent.flags = 0;
					count += 1;
					
					paddr += ent_bytes as ::kernel::memory::PAddr;
					bytes -= ent_bytes;
				}
			}
		}
		assert!(count > 0);
		self.prdts[count-1].flags = 0x8000;
		//Ok( () )
	}
	
	fn start_dma(&mut self, disk: u8, blockidx: u64, dma_buffer: &DMABuffer, is_write: bool, bm: &DmaRegBorrow)
	{
		log_debug!("start_dma(disk={},blockidx={},is_write={},dma_buffer={{len={}}})",
			disk, blockidx, is_write, dma_buffer.len());
		let count = dma_buffer.len() / SECTOR_SIZE;
		
		self.fill_prdt(dma_buffer);
		
		// Commence the IO and return a wait handle for the operation
		// SAFE: Unique access and valid IO accesses
		unsafe
		{
			// - Only use LBA48 if needed
			if blockidx >= (1 << 28) || count >= 256
			{
				self.out_8(6, 0x40 | (disk << 4));
				self.out_8(2, (count >> 8) as u8);
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
	
	fn start_atapi(&mut self, bm: &DmaRegBorrow, disk: u8, is_write: bool, cmd: &[u16], dma_buffer: &DMABuffer)
	{
		log_debug!("start_atapi(...,disk={},is_write={},cmd={{len={}}},dma_buffer={{len={}}})",
			disk, is_write, cmd.len()*2, dma_buffer.len());
		//log_debug!("- cmd=[{:02x},{:02x},{:02x},{:02x},{:02x},{:02x},{:02x},{:02x},{:02x},{:02x},{:02x},{:02x}]",
		//	cmd[0] & 0xFF, cmd[0] >> 8,
		//	cmd[1] & 0xFF, cmd[1] >> 8,
		//	cmd[2] & 0xFF, cmd[2] >> 8,
		//	cmd[3] & 0xFF, cmd[3] >> 8,
		//	cmd[4] & 0xFF, cmd[4] >> 8,
		//	cmd[5] & 0xFF, cmd[5] >> 8
		//	);
		
		self.fill_prdt(dma_buffer);
		
		// Commence the IO and return a wait handle for the operation
		// SAFE: Locked (unique self) and checked access
		unsafe
		{
			// - Set PRDT
			bm.out_32(4, ::kernel::memory::virt::get_phys(&self.prdts[0]) as u32);
			bm.out_8(0, 0x04);	// Reset IRQ
			// Start IO
			bm.out_8(0, if is_write { 0 } else { 8 } | 1);

			// Select channel
			self.out_8(6, disk << 4);
			// Set DMA enable
			self.out_8(1, 0x01);
			// Max byte count
			self.out_8(4, (dma_buffer.len() >> 0) as u8);
			self.out_8(5, (dma_buffer.len() >> 8) as u8);
			// ATAPI PACKET
			self.out_8(7, 0xA0);

			// - Send command once IRQ is fired?
			// TODO: Find a way of avoiding this poll (extra wait state)

			while self.in_sts() & AtaStatusVal::BSY != 0 { }
			self.atapi_send_cmd(cmd);
		}
	}

	fn atapi_send_cmd(&mut self, cmd: &[u16])
	{
		// Command must be 6 words long
		assert!(cmd.len() == 6);
		// SAFE: Unique self
		unsafe {
			assert!(self.in_sts() & AtaStatusVal::DRQ != 0);
			
			// Send command
			for &word in cmd {
				self.out_16(0, word);
			}
		}
	}
}

/*
enum WaitState<'dev>
{
	Acquire(async::mutex::Waiter<'dev,AtaRegs>),
	IoActive(async::mutex::HeldMutex<'dev,AtaRegs>, async::event::Waiter<'dev>),
	Done(Result<(),storage::IoError>),
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
impl<'a,'b> async::ResultWaiter for AtaWaiter<'a,'b>
{
	type Result = Result<usize, storage::IoError>;
	
	fn get_result(&mut self) -> Option<Self::Result> {
		match self.state
		{
		WaitState::Done(r) => Some(r.map( |()| self.dma_buffer.len() / 512 )),
		//WaitState::Done(r) => Some(r),
		_ => None,
		}
	}
	
	fn as_waiter(&mut self) -> &mut dyn async::Waiter { self }
}

impl<'a,'b> async::Waiter for AtaWaiter<'a,'b>
{
	fn is_complete(&self) -> bool {
		if let WaitState::Done(..) = self.state { true } else { false }
	}
	
	fn get_waiter(&mut self) -> &mut dyn async::PrimitiveWaiter
	{
		match self.state
		{
		// Initial state: Acquire the register lock
		WaitState::Acquire(ref mut waiter) => waiter,
		// Final state: Start IO and wait for it to complete
		WaitState::IoActive(_, ref mut waiter) => waiter,
		//
		WaitState::Done(..) => unreachable!(),
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
			WaitState::IoActive(ref mut lh, ref _waiter) => WaitState::Done(
				// SAFE: Holding the register lock
				unsafe {
					log_trace!("Complete");
					self.dma_regs.out_8(0, 0);	// Stop transfer
					let ata_status = AtaStatusVal(lh.in_8(7));
					let dma_status = DmaStatusVal(self.dma_regs.in_8(2));
					log_trace!("BM Status = {:?}, ATA Status = {:?}", dma_status, ata_status);
					lh.last_result(false)	// not ATAPI
				}
				),
			//
			WaitState::Done(..) => unreachable!(),
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
		WaitState::Done(..) => write!(f, "(Done)"),
		}
	}
}
struct AtapiWaiter<'dev,'buf>
{
	dev: &'dev AtaController,
	disk: u8,
	is_write: bool,
	dma_regs: DmaRegBorrow<'dev>,
	cmd_buffer: [u16; 6],
	dma_buffer: DMABuffer<'buf>,
	state: WaitState<'dev>,
}
impl<'a,'b> async::ResultWaiter for AtapiWaiter<'a,'b>
{
	type Result = Result<(), storage::IoError>;
	
	fn get_result(&mut self) -> Option<Self::Result> {
		match self.state
		{
		WaitState::Done(r) => Some(r),
		_ => None,
		}
	}
	
	fn as_waiter(&mut self) -> &mut dyn async::Waiter { self }
}

impl<'a,'b> async::Waiter for AtapiWaiter<'a,'b>
{
	fn is_complete(&self) -> bool {
		if let WaitState::Done(..) = self.state { true } else { false }
	}
	
	fn get_waiter(&mut self) -> &mut dyn async::PrimitiveWaiter
	{
		match self.state
		{
		// Initial state: Acquire the register lock
		WaitState::Acquire(ref mut waiter) => waiter,
		// Final state: Start IO and wait for it to complete
		WaitState::IoActive(_, ref mut waiter) => waiter,
		//
		WaitState::Done(..) => unreachable!(),
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
				lh.start_atapi( &self.dma_regs, self.disk, self.is_write, &self.cmd_buffer, &self.dma_buffer );
				WaitState::IoActive(lh, self.dev.interrupt.handle.get_event().wait())
				},
			// And if IoActive completes, we're complete
			WaitState::IoActive(ref mut lh, ref _waiter) => {
				// If the controller is still busy, keep going
				if lh.in_sts() & AtaStatusVal::BSY != 0 {
					log_warning!("Controller still busy when waiter woken");
					return false;
				}
				// SAFE: Holding the register lock
				let completion_res = unsafe {
						//log_trace!("Complete");
						self.dma_regs.out_8(0, 0);	// Stop transfer
						let ata_status = AtaStatusVal( lh.in_8(7) );
						let dma_status = DmaStatusVal(self.dma_regs.in_8(2));
						log_trace!("BM Status = {:?}, ATA Status = {:?}", dma_status, ata_status);
						lh.last_result(true)
					};
				WaitState::Done( completion_res )
				},
			//
			WaitState::Done(..) => unreachable!(),
			};
		
		self.is_complete()
	}
}
impl<'a,'b> ::core::fmt::Debug for AtapiWaiter<'a,'b> {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		try!( write!(f, "AtapiWaiter") );
		match self.state
		{
		WaitState::Acquire(..) => write!(f, "(Acquire)"),
		WaitState::IoActive(..) => write!(f, "(IoActive)"),
		WaitState::Done(..) => write!(f, "(Done)"),
		}
	}
}
*/

impl AtaController
{
	pub fn new(ata_base: u16, sts_port: u16, irq: u32) -> AtaController
	{
		AtaController {
			regs: ::kernel::futures::Mutex::new( AtaRegs::new(ata_base, sts_port) ),
			interrupt: AtaInterrupt {
				handle: ::kernel::irqs::bind_event(irq),
				},
			}
	}
	
	async fn do_dma<'a,'b>(&'a self, blockidx: u64, dst: DMABuffer<'b>, disk: u8, is_write: bool, dma_regs: DmaRegBorrow<'a>) -> Result<usize,storage::IoError>
	{
		let mut lh = self.regs.async_lock().await;
		lh.start_dma( disk, blockidx, &dst, is_write, &dma_regs );
		self.interrupt.handle.get_event().wait().await;

		// SAFE: Holding the register lock
		unsafe {
			log_trace!("Complete");
			dma_regs.out_8(0, 0);	// Stop transfer
			let ata_status = AtaStatusVal(lh.in_8(7));
			let dma_status = DmaStatusVal(dma_regs.in_8(2));
			log_trace!("BM Status = {:?}, ATA Status = {:?}", dma_status, ata_status);
			lh.last_result(false)?	// not ATAPI
		}

		Ok( dst.len() / 512 )
	}
	fn do_atapi<'a>(&'a self, disk: u8, dma_regs: DmaRegBorrow<'a>, cmd: &[u8], dst: DMABuffer<'a>, is_write: bool)
		-> impl ::core::future::Future<Output=Result<(), storage::IoError>> + 'a
	{
		let cmd_buffer = {
			let mut buf = [0u16; 6];
			for i in 0 .. 6 {
				// Read zero-padded little endian words from stream
				let w = if i*2+1 < cmd.len() {
						cmd[i*2] as u16 | ((cmd[i*2+1] as u16) << 8)
					}
					else if i*2+1 == cmd.len() {
						cmd[cmd.len()-1] as u16
					}
					else {
						0
					};
				buf[i] = w;
			}
			buf
			};
		self.do_atapi_inner(disk, dma_regs, cmd_buffer, dst, is_write)
	}
	async fn do_atapi_inner(&self, disk: u8, dma_regs: DmaRegBorrow<'_>, cmd_buffer: [u16; 6], dst: DMABuffer<'_>, is_write: bool) -> Result<(), storage::IoError>
	{
		let mut lh = self.regs.async_lock().await;
		lh.start_atapi( &dma_regs, disk, is_write, &cmd_buffer, &dst );
		loop
		{
			self.interrupt.handle.get_event().wait().await;

			// If the controller is still busy, keep going
			if lh.in_sts() & AtaStatusVal::BSY != 0 {
				log_warning!("Controller still busy when waiter woken");
				continue;
			}
			break;
		}

		// SAFE: Holding the register lock
		let completion_res = unsafe {
				//log_trace!("Complete");
				dma_regs.out_8(0, 0);	// Stop transfer
				let ata_status = AtaStatusVal( lh.in_8(7) );
				let dma_status = DmaStatusVal( dma_regs.in_8(2) );
				log_trace!("BM Status = {:?}, ATA Status = {:?}", dma_status, ata_status);
				lh.last_result(true)
			};
		
		completion_res
	}
	
	/// Request an ATA IDENTIFY packet from the device
	pub async fn ata_identify<'a>(&'a self, disk: u8, data: &'a mut crate::AtaIdentifyData, class: &'a mut crate::AtaClass)
	{
		// - Cast 'data' to a u16 slice
		// SAFE: AtaIdentifyData should be POD, and has at least `u16` alignment
		let data: &mut [u16; 256] = unsafe { ::core::mem::transmute(data) };
		let mut buslock = self.regs.async_lock().await;
	
		log_debug!("ata_identify: (disk={}), base={:#x}", disk, buslock.ata_base);
		// SAFE: Called holding lock, and performs correct actions
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
			*class = crate::AtaClass::None;
			// SAFE: Plain old data
			*data = unsafe { ::core::mem::zeroed() };
		}
		else
		{
			// Block until BSY clears
			// TODO: Timeout?
			while buslock.in_sts() & AtaStatusVal::BSY != 0 { }
			
			while buslock.in_sts() & (AtaStatusVal::DRQ | AtaStatusVal::ERR) == 0 {
				::kernel::futures::msleep(1).await;
			}

			if buslock.in_sts() & 1 == 1 {
				// - Error, clear and return
				// SAFE: Called holding the lock
				let (f4, f5) = unsafe { (buslock.in_8(4), buslock.in_8(5)) };
				// SAFE: Plain old data
				*data = unsafe { ::core::mem::zeroed() };
				if f4 == 0x14 && f5 == 0xEB {
					// Device is ATAPI
					log_debug!("ata_identify: Disk {:#x}/{} is ATAPI", buslock.ata_base, disk);
					*class = crate::AtaClass::ATAPI;
				}
				else {
					log_debug!("ata_identify: Disk {:#x}/{} errored (f4,f5 = {:#02x},{:#02x})", buslock.ata_base, disk, f4, f5);
					*class = crate::AtaClass::Unknown(f4, f5);
				}
			}
			else {
				// Success, perform IO
				buslock.read_sector(data);
				log_debug!("ata_identify: Disk {:#x}/{} IDENTIFY complete", buslock.ata_base, disk);
				*class = crate::AtaClass::Native;
			}
		}
	}
}

impl_fmt! {
	Debug(self,f) for DmaStatusVal {{
		write!(f, "({:#x}", self.0)?;
		if self.0 & (1<<0) != 0 { write!(f, " DMAing")?; }
		if self.0 & (1<<1) != 0 { write!(f, " Fail")?; }
		if self.0 & (1<<2) != 0 { write!(f, " IRQ")?; }
		if self.0 & (1<<5) != 0 { write!(f, " MasterS")?; }
		if self.0 & (1<<6) != 0 { write!(f, " SlaveS")?; }
		if self.0 & (1<<7) != 0 { write!(f, " SO")?; }
		write!(f, ")")
	}}
}
impl AtaStatusVal
{
	const ERR: u8 = (1<<0);	// Set on error
	const DRQ: u8 = (1<<3);	// Expecting PIO
	const SRV: u8 = (1<<4);	// Overlapped service request
	const DF:  u8 = (1<<5);	// Drive fault
	const RDY: u8 = (1<<6);	// Set when the drive is ready
	const BSY: u8 = (1<<7);	// Drive is busy prepping for IO
}
impl_fmt! {
	Debug(self,f) for AtaStatusVal {{
		write!(f, "({:#x}", self.0)?;
		if self.0 & Self::ERR != 0 { write!(f, " ERR")?; }
		if self.0 & Self::DRQ != 0 { write!(f, " DRQ")?; }
		if self.0 & Self::SRV != 0 { write!(f, " SRV")?; }
		if self.0 & Self::DF  != 0 { write!(f, " DF" )?; }
		if self.0 & Self::RDY != 0 { write!(f, " RDY")?; }
		if self.0 & Self::BSY != 0 { write!(f, " BSY")?; }
		write!(f, ")")
	}}
}
impl AtaErrorVal
{
	const MARK: u8 = (1<<0);	// Bad address mark
	const TRK0: u8 = (1<<1);	// Cannot find track 0
	const ABRT: u8 = (1<<2);	// Operation aborted (command not supported)
	const MCR:  u8 = (1<<3);	// Media change request
	const ID:   u8 = (1<<4);	// ID field not found
	const MC:   u8 = (1<<5);	// Media changed
	const ECC:  u8 = (1<<6);	// Uncorrectable ECC
	const ICRC: u8 = (1<<7);	// CRC error (or bad block, pre-EIDE)
}
impl_fmt! {
	Debug(self,f) for AtaErrorVal {{
		write!(f, "({:#x}", self.0)?;
		if self.0 & Self::MARK != 0 { write!(f, " MARK")?; }
		if self.0 & Self::TRK0 != 0 { write!(f, " TRK0")?; }
		if self.0 & Self::ABRT != 0 { write!(f, " ABRT")?; }
		if self.0 & Self::MCR  != 0 { write!(f, " MCR" )?; }
		if self.0 & Self::ID   != 0 { write!(f, " ID"  )?; }
		if self.0 & Self::MC   != 0 { write!(f, " MC"  )?; }
		if self.0 & Self::ECC  != 0 { write!(f, " ECC" )?; }
		if self.0 & Self::ICRC != 0 { write!(f, " ICRC")?; }
		write!(f, ")")
	}}
}
impl AtapiErrorVal
{
	const NO_SENSE:        u8 = 0;
	const RECOVERED_ERROR: u8 = 1;
	const NOT_READY:       u8 = 2;
	const MEDIUM_ERROR:    u8 = 3;
	const HARDWARE_ERROR:  u8 = 4;
	const ILLEGAL_REQUEST: u8 = 5;
	const UNIT_ATTENTION:  u8 = 6; 
	const DATA_PROTECT:    u8 = 7;
	const BLANK_CHECK:     u8 = 8;
	//                          9
	const COPY_ABORTED:    u8 = 10;
	const ABORTED_COMMAND: u8 = 11;
	//                          12
	const VOLUME_OVERFLOW: u8 = 13;
	const MISCOMPARE:      u8 = 14;
	
	fn sense_key(&self) -> u8 { self.0 >> 4 }
}
impl_fmt! {
	Debug(self,f) for AtapiErrorVal {{
		write!(f, "({:#x}", self.0)?;
		write!(f, " {}", match self.sense_key() {
			Self::NO_SENSE        => "NO_SENSE",
			Self::RECOVERED_ERROR => "RECOVERED_ERROR",
			Self::NOT_READY       => "NOT_READY",
			Self::MEDIUM_ERROR    => "MEDIUM_ERROR",
			Self::HARDWARE_ERROR  => "HARDWARE_ERROR",
			Self::ILLEGAL_REQUEST => "ILLEGAL_REQUEST",
			Self::UNIT_ATTENTION  => "UNIT_ATTENTION",
			Self::DATA_PROTECT    => "DATA_PROTECT",
			Self::BLANK_CHECK     => "BLANK_CHECK",
			9 => "unk9",
			Self::COPY_ABORTED    => "COPY_ABORTED",
			Self::ABORTED_COMMAND => "ABORTED_COMMAND",
			12 => "unk12",
			Self::VOLUME_OVERFLOW => "VOLUME_OVERFLOW",
			Self::MISCOMPARE      => "MISCOMPARE",
			15 => "unk15",
			_ => "invalid",
			})?;
		write!(f, ")")
	}}
}

