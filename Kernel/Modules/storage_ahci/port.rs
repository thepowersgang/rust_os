//
//
//
//! 
use kernel::prelude::*;
use core::sync::atomic::{Ordering,AtomicU32};
use kernel::sync::Mutex;
use kernel::metadevs::storage::{self, DataPtr};
use kernel::memory::virt::AllocHandle;
use kernel::lib::mem::aref::ArefBorrow;
use kernel::device_manager;
use hw;

enum Error
{
	Ata { err: u8, sts: u8 },
	Atapi { sense_key: ::storage_scsi::proto::SenseKey, eom: bool, ili: bool },
	Bus,
}
impl_fmt! {
	Debug(self,f) for Error {
		match self
		{
		&Error::Ata { err, sts } => write!(f, "Ata(sts={:02x} err={:#x}{})", sts, err,
			if err & (1 << 2) != 0 { " ABRT" } else { "" }
			),
		&Error::Atapi { sense_key, eom, ili } => write!(f, "Atapi(sense_key={:?},eom={},ili={})", sense_key, eom, ili),
		&Error::Bus => write!(f, "Bus"),
		}
	}
}

pub struct Port
{
	name: String,
	pub index: usize,
	ctrlr: ArefBorrow<::controller::ControllerInner>,

	volume: Mutex<Option<storage::PhysicalVolumeReg>>,
	
	// Hardware allocations:
	// - 1KB (<32*32 bytes) for the command list
	// - 256 bytes of Received FIS
	// - <16KB (32*256 bytes) of command tables
	// Contains the "Command List" (a 1KB aligned block of memory containing commands)
	command_list_alloc: AllocHandle,
	command_tables: [Option<AllocHandle>; 4],

	command_events: Vec<::kernel::sync::EventChannel>,

	used_commands_sem: ::kernel::sync::Semaphore,
	used_commands: AtomicU32,
}
pub struct PortRegs<'a>
{
	idx: usize,
	io: &'a device_manager::IOBinding,
}

impl<'a> PortRegs<'a>
{
	pub fn new(io: &device_manager::IOBinding, port_idx: usize) -> PortRegs {
		PortRegs {
			idx: port_idx,
			io: io
			}
	}

	pub fn read(&self, ofs: usize) -> u32 {
		assert!(ofs < 0x80);
		assert!(ofs & 3 == 0);
		// SAFE: None of the Px registers have a read side-effect
		unsafe { self.io.read_32(hw::REG_Px + self.idx * 0x80 + ofs) }
	}
	pub unsafe fn write(&self, ofs: usize, val: u32) {
		assert!(ofs < 0x80);
		self.io.write_32(hw::REG_Px + self.idx * 0x80 + ofs, val)
	}
}

// Maximum number of commands before a single page can't be shared
const MAX_COMMANDS_FOR_SHARE: usize = (::kernel::PAGE_SIZE - 256) / (256 + 32);
const CMDS_PER_PAGE: usize = ::kernel::PAGE_SIZE / 0x100;


impl ::core::fmt::Display for Port
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		write!(f, "AHCI ? Port {}", self.index)
	}
}
impl Port
{
	/// UNSAFE: Caller shall ensure that:
	/// - `idx` is unique
	/// - The stored instance does not move once any other methods are called.
	pub unsafe fn new(controller: ArefBorrow<::controller::ControllerInner>, idx: usize, max_commands: usize) -> Result<Port, device_manager::DriverBindError>
	{
		use core::mem::size_of;
		log_trace!("Port::new(, idx={}, max_commands={})", idx, max_commands);

		assert!(idx < 32);
		assert!(max_commands <= 32);
		
		let (cl_page, cmdtab_pages) = try!( Self::allocate_memory(&controller) );

		// Populate register values.
		{
			let regs = PortRegs::new(&controller.io_base, idx);

			let addr = ::kernel::memory::virt::get_phys( cl_page.as_ref::<()>(0) ) as u64;
			regs.write(hw::REG_PxCLB , (addr >>  0) as u32);
			regs.write(hw::REG_PxCLBU, (addr >> 32) as u32);
			let addr = ::kernel::memory::virt::get_phys( cl_page.as_ref::<hw::RcvdFis>( ::kernel::PAGE_SIZE - size_of::<hw::RcvdFis>() ) ) as u64;
			regs.write(hw::REG_PxFB , (addr >>  0) as u32);
			regs.write(hw::REG_PxFBU, (addr >> 32) as u32);

			// Clear PxACT (TODO: not really used here)
			regs.write(hw::REG_PxSACT, 0);
			// Interrupts on
			regs.write(hw::REG_PxSERR, 0x3FF783);
			regs.write(hw::REG_PxIS, !0);
			regs.write(hw::REG_PxIE, hw::PxIS_CPDS|hw::PxIS_DSS|hw::PxIS_PSS|hw::PxIS_DHRS|hw::PxIS_TFES|hw::PxIS_IFS);
			// Start command engine (Start, FIS Rx Enable)
			let cmd = regs.read(hw::REG_PxCMD);
			regs.write(hw::REG_PxCMD, cmd|hw::PxCMD_ST|hw::PxCMD_FRE);
		}


		Ok(Port {
			name: format!("ahci?-{}", idx),
			ctrlr: controller,
			index: idx,

			volume: Mutex::new(None),

			command_list_alloc: cl_page,
			command_tables: cmdtab_pages,

			command_events: (0 .. max_commands).map(|_| ::kernel::sync::EventChannel::new()).collect(),
			used_commands_sem: ::kernel::sync::Semaphore::new(max_commands as isize, max_commands as isize),
			used_commands: AtomicU32::new(0),
			})
	}
	
	fn allocate_memory(controller: &::controller::ControllerInner) -> Result< (AllocHandle, [Option<AllocHandle>; 4]), device_manager::DriverBindError >
	{
		use core::mem::size_of;
		let max_commands = controller.max_commands as usize;
		let cl_size = max_commands * size_of::<hw::CmdHeader>();

		// Command list
		// - Command list first (32 * max_commands)
		// - Up to MAX_COMMANDS_FOR_SHARE in 1024 -- 4096-256
		// - RcvdFis last
		let cl_page = try!( ::kernel::memory::virt::alloc_dma(64, 1, "AHCI") );

		// Allocate pages for the command table
		// TODO: Delay allocating memory until a device is detected on this port
		let cmdtab_pages = if max_commands <= MAX_COMMANDS_FOR_SHARE {
				// All fits in the CL page!
				
				// - Return empty allocations
				Default::default()
			}
			else {
				// Individual pages for the command table, but the RcvdFis and CL share
				let mut tab_pages: [Option<AllocHandle>; 4] = Default::default();
				let n_pages = (max_commands - MAX_COMMANDS_FOR_SHARE + CMDS_PER_PAGE-1) / CMDS_PER_PAGE;
				assert!(n_pages < 4);
				for i in 0 .. n_pages
				{
					tab_pages[i] = Some( ::kernel::memory::virt::alloc_dma(64, 1, "AHCI")? );
				}
				tab_pages
			};


		// Initialise the command list and table entries
		{
			// SAFE: Doesn't alias, as we uniquely own cl_page, and cmdidx_to_ref should be correct.
			let cl_ents = unsafe { cl_page.as_int_mut_slice(0, max_commands) };
			for (listent, tabent) in Iterator::zip( cl_ents.iter_mut(), (0 .. max_commands).map(|i| Self::cmdidx_to_ref(&cl_page, cl_size, &cmdtab_pages, i)) )
			{
				*listent = hw::CmdHeader::new( ::kernel::memory::virt::get_phys(tabent) as u64 );
				//*tabent = hw::CmdTable::new();
			}
		}

		Ok( (cl_page, cmdtab_pages) )
	}


	pub fn handle_irq(&self)
	{
		let regs = self.regs();

		let int_status = regs.read(hw::REG_PxIS);
		let tfd = regs.read(hw::REG_PxTFD);
		//log_trace!("{} - int_status={:#x}", self, int_status);

		// Cold Port Detection Status
		if int_status & hw::PxIS_CPDS != 0
		{
			log_notice!("{} - Presence change", self);
		}


		// "Task File Error Status"
		if int_status & hw::PxIS_TFES != 0
		{
			log_warning!("{} - Device pushed error: TFD={:#x}", self, tfd);
			// TODO: This should terminate all outstanding transactions with an error.
		}

		// Device->Host Register Update
		if int_status & hw::PxIS_DHRS != 0
		{
			log_trace!("{} - Device register update, RFIS={:?}", self, self.get_rcvd_fis().RFIS);
		}
		// PIO Setup FIS Update
		if int_status & hw::PxIS_PSS != 0
		{
			log_trace!("{} - PIO setup status update, PSFIS={:?}", self, self.get_rcvd_fis().PSFIS);
		}

		if int_status & hw::PxIS_IFS != 0
		{
		}

		// Check commands
		//if int_status & hw::PxIS_DPS != 0
		//{
		let issued_commands = regs.read(hw::REG_PxCI);
		let active_commands = regs.read(hw::REG_PxSACT);
		let used_commands = self.used_commands.load(Ordering::Relaxed);
		//log_trace!("{} - used_commands = {:#x}, issued_commands={:#x}, active_commands={:#x}",
		//	self, used_commands, issued_commands, active_commands);
		for cmd in 0 .. self.ctrlr.max_commands as usize
		{
			let mask = 1 << cmd;
			if used_commands & mask != 0
			{
				if tfd & 0x01 != 0 {
					self.command_events[cmd].post();
				}
				else if issued_commands & mask == 0 || active_commands & mask == 0 {
					self.command_events[cmd].post();
				}
				else {
					// Not yet complete
				}
			}
			else if active_commands & mask != 0	{
				log_warning!("{} - Command {} active, but not used", self, cmd);
			}
			else {
			}
		}
		//}
	
		// SAFE: Exclusive range, only written here
		unsafe {
			regs.write(hw::REG_PxIS, int_status);
		}
	}

	fn get_rcvd_fis(&self) -> &hw::RcvdFis
	{
		self.command_list_alloc.as_ref::<hw::RcvdFis>( ::kernel::PAGE_SIZE - ::core::mem::size_of::<hw::RcvdFis>() )
	}

	fn cmdidx_to_ref<'a>(cl_page: &'a AllocHandle, cl_size: usize, cmdtab_pages: &'a [Option<AllocHandle>], i: usize) -> &'a hw::CmdTable {
		//let cl_size = max_commands * size_of::<hw::CmdHeader>();
		let n_shared = (::kernel::PAGE_SIZE - cl_size) / 0x100 - 1;
		if i < n_shared {
			&cl_page.as_slice(cl_size, n_shared)[i]
		}
		else {
			let i = i - n_shared;
			let (pg,ofs) = (i / CMDS_PER_PAGE, i % CMDS_PER_PAGE);
			&cmdtab_pages[pg].as_ref().expect("Index above shared threshold, but not present").as_slice(0, CMDS_PER_PAGE)[ofs]
		}
	}
	fn get_cmdtab_ptr(&self, idx: usize) -> *mut hw::CmdTable
	{
		// TODO: Does the fact that this returns &-ptr break anything?
		let r = Self::cmdidx_to_ref(&self.command_list_alloc, self.ctrlr.max_commands as usize * ::core::mem::size_of::<hw::CmdHeader>(), &self.command_tables,  idx);
		r as *const _ as *mut _
	}

	fn regs(&self) -> PortRegs {
		PortRegs {
			idx: self.index,
			io: &self.ctrlr.io_base,
			}
	}

	// Re-check the port for a new device
	pub fn update_connection(&self)
	{
		let io = self.regs();

		// SAFE: Status only registers
		let (tfd, ssts) = (io.read(hw::REG_PxTFD), io.read(hw::REG_PxSSTS));

		if tfd & (hw::PxTFD_STS_BSY|hw::PxTFD_STS_DRQ) != 0 {
			return ;
		}
		// SATA Status: Detected. 3 = Connected and PHY up
		if (ssts & hw::PxSSTS_DET) >> hw::PxSSTS_DET_ofs != 3 {
			return ;
		}
		

		// Obtain the physical volume registration handle
		let pvh = match io.read(hw::REG_PxSIG)
			{
			// Standard ATA
			0x00000101 => {
				// Request ATA Identify from the disk
				const ATA_IDENTIFY: u8 = 0xEC;
				let ident = self.request_identify(ATA_IDENTIFY).expect("Failure requesting ATA identify");

				log_debug!("ATA `IDENTIFY` response data = {:?}", ident);
				
				let sectors = if ident.sector_count_48 == 0 { ident.sector_count_28 as u64 } else { ident.sector_count_48 };
				log_log!("{}: Hard Disk, {} sectors, {}", self, sectors, storage::SizePrinter(sectors * 512));

				//*
				match ::storage_ata::volume::AtaVolume::new_boxed( self.get_interface() )
				{
				Ok(vol) => Some(storage::register_pv(vol)),
				Err(e) => { log_error!("{}: Error while creating ATA device: {:?}", self, e); None },
				}
				// */
				//None
				},
			// ATAPI Device
			0xEB140101 => {
				//const ATA_IDENTIFY_PACKET: u8 = 0xA1;
				//let ident = self.request_identify(ATA_IDENTIFY_PACKET).expect("Failure requesting ATA IDENTIFY PACKET");
				//log_debug!("ATA `IDENTIFY_PACKET_DEVICE` response data = {:?}", ident);

				log_log!("{}: ATAPI Device", self);
				match ::storage_scsi::Volume::new_boxed( self.get_interface() )
				{
				Ok(scsi_vol) => Some(storage::register_pv(scsi_vol)),
				Err(e) => { log_error!("{}: Error while creating SCSI device: {:?}", self, e); None },
				}
				},
			// Unknown - Log an error
			signature @ _ => {
				log_error!("{} - Unknown signature {:08x}", self, signature);
				None
				},
			};

		let mut lh = self.volume.lock();
		if lh.is_some() {
			log_warning!("{} - A volume is already registered", self);
		}
		*lh = pvh;
	}

	fn get_interface(&self) -> Interface {
		// TODO: Store a reference count locally that's decremented when Interface is dropped

		// SAFE: Self::new() requires that this object not be moved once any methods are called. Lifetime controlled by the volume handle
		unsafe {
			Interface::new(self)
		}
	}

	fn request_identify(&self, cmd: u8) -> Result<::storage_ata::AtaIdentifyData, Error>
	{
		let mut ata_identify_data = ::storage_ata::AtaIdentifyData::default();
		try!( self.request_ata_lba28(0, cmd, 0,0, DataPtr::Recv(::kernel::lib::as_byte_slice_mut(&mut ata_identify_data))) );

		fn flip_bytes(bytes: &mut [u8]) {
			for pair in bytes.chunks_mut(2) {
				pair.swap(0, 1);
			}
		}
		// All strings are sent 16-bit endian flipped, so reverse that
		flip_bytes(&mut ata_identify_data.serial_number);
		flip_bytes(&mut ata_identify_data.firmware_ver);
		flip_bytes(&mut ata_identify_data.model_number);
		Ok( ata_identify_data )
	}

	fn request_ata_lba28(&self, disk: u8, cmd: u8,  n_sectors: u8, lba: u32, data: DataPtr) -> Result<usize, Error>
	{
		log_trace!("request_ata_lba28(disk={}, cmd={:#02x}, n_sectors={}, lba={})", disk, cmd, n_sectors, lba);
		assert!(lba < (1<<24));
		let cmd_data = hw::sata::FisHost2DevReg {
			ty: hw::sata::FisType::H2DRegister as u8,
			flags: 0x80,
			command: cmd,
			sector_num: lba as u8,
			cyl_low: (lba >> 8) as u8,
			cyl_high: (lba >> 16) as u8,
			dev_head: 0x40 | (disk << 4) | (lba >> 24) as u8,
			sector_num_exp: 0,
			sector_count: n_sectors,
			sector_count_exp: 0,
			..Default::default()
			};
		self.do_fis(cmd_data.as_ref(), &[], data)
	}
	fn request_ata_lba48(&self, disk: u8, cmd: u8,  n_sectors: u16, lba: u64, data: DataPtr) -> Result<usize, Error>
	{
		log_trace!("request_ata_lba48(disk={}, cmd={:#02x}, n_sectors={}, lba={})", disk, cmd, n_sectors, lba);
		assert!(lba < (1<<48));
		let cmd_data = hw::sata::FisHost2DevReg {
			ty: hw::sata::FisType::H2DRegister as u8,
			flags: 0x80,
			command: cmd,
			sector_num: lba as u8,
			cyl_low: (lba >> 8) as u8,
			cyl_high: (lba >> 16) as u8,
			dev_head: 0x40 | (disk << 4),
			sector_num_exp: (lba >> 24) as u8,
			cyl_low_exp: (lba >> 32) as u8,
			cyl_high_exp: (lba >> 40) as u8,
			sector_count: n_sectors as u8,
			sector_count_exp: (n_sectors >> 8) as u8,
			..Default::default()
			};
		self.do_fis(cmd_data.as_ref(), &[], data)
	}
	fn request_atapi(&self, disk: u8, cmd: &[u8], data: DataPtr) -> Result<(), Error>
	{
		let fis = hw::sata::FisHost2DevReg {
			ty: hw::sata::FisType::H2DRegister as u8,
			flags: 0x80,
			command: 0xA0,
			dev_head: (disk << 4),
			cyl_low: (data.len() & 0xFF) as u8,
			cyl_high: (data.len() >> 8) as u8,
			..Default::default()
			};
		match self.do_fis(fis.as_ref(), cmd, data)
		{
		Ok(_) => Ok( () ),
		Err(e) => Err(e),
		}
	}

	/// Create and dispatch a FIS, returns the number of bytes
	fn do_fis(&self, cmd: &[u8], pkt: &[u8], data: DataPtr) -> Result<usize, Error>
	{
		use kernel::memory::virt::get_phys;

		//log_trace!("do_fis(self={}, cmd={:p}+{}, pkt={:p}+{}, data={:?})",
		//	self, cmd.as_ptr(), cmd.len(), pkt.as_ptr(), pkt.len(), data);

		let slot = self.get_command_slot();

		slot.data.cmd_fis[..cmd.len()].clone_from_slice(cmd);
		slot.data.atapi_cmd[..pkt.len()].clone_from_slice(pkt);

		// Generate the scatter-gather list
		let mut va = data.as_slice().as_ptr() as usize;
		let mut len = data.as_slice().len();
		let mut n_prdt_ents = 0;
		while len > 0
		{
			let base_phys = get_phys(va as *const u8);
			let mut seglen = ::kernel::PAGE_SIZE - base_phys as usize % ::kernel::PAGE_SIZE;
			const MAX_SEG_LEN: usize = 1 << 22;
			// Each entry must be contigious, and not >4MB
			while seglen < len && seglen <= MAX_SEG_LEN && get_phys( (va + seglen-1) as *const u8 ) == base_phys + (seglen-1) as ::kernel::memory::PAddr
			{
				seglen += ::kernel::PAGE_SIZE;
			}
			let seglen = ::core::cmp::min(len, seglen);
			let seglen = ::core::cmp::min(MAX_SEG_LEN, seglen);
			if base_phys % 4 != 0 || seglen % 2 != 0 {
				todo!("AHCI Port::do_fis - Use a bounce buffer if alignment requirements are not met");
			}
			assert!( n_prdt_ents < slot.data.prdt.len() );
			slot.data.prdt[n_prdt_ents].dba = base_phys as u64;
			slot.data.prdt[n_prdt_ents].dbc = (seglen - 1) as u32;

			va += seglen;
			len -= seglen;

			n_prdt_ents += 1;
		}
		slot.data.prdt[n_prdt_ents-1].dbc |= 1 << 31;	// set IOC
		slot.hdr.prdtl = n_prdt_ents as u16;
		slot.hdr.prdbc = 0;
		slot.hdr.flags = (cmd.len() / 4) as u16
			//| (multiplier_port << 12)
			| (if data.is_send() { 1 << 6 } else { 0 })	// Write
			| (if pkt.len() > 0 { 1 << 5 } else { 0 })	// ATAPI
			;

		slot.event.clear();
		// SAFE: Wait ensures that memory stays valid
		unsafe {
			slot.start();
			slot.wait()
		}
	}

	fn get_command_slot(&self) -> CommandSlot
	{
		let max_commands = self.ctrlr.max_commands as usize;

		// 0. Request slot from semaphore
		self.used_commands_sem.acquire();
		
		// 1. Load
		let mut cur_used_commands = self.used_commands.load(Ordering::Relaxed);
		loop
		{
			// 2. Search
			let mut avail = self.ctrlr.max_commands as usize;
			for i in 0 .. self.ctrlr.max_commands as usize
			{
				if cur_used_commands & 1 << i == 0 {
					avail = i;
					break ;
				}
			}
			assert!(avail < self.ctrlr.max_commands as usize);

			// 3. Try and commit
			// - Can't use `fetch_or` because that wouldn't spot races
			let try_new_val = cur_used_commands | (1 << avail);
			if let Err(newval) = self.used_commands.compare_exchange(cur_used_commands, try_new_val, Ordering::Acquire, Ordering::Relaxed)
			{
				cur_used_commands = newval;
				continue ;
			}
			// If successful, return
			// SAFE: Exclusive access
			let (tab, hdr) = unsafe {
				(
					&mut *self.get_cmdtab_ptr(avail),
					&mut self.command_list_alloc.as_int_mut_slice(0, max_commands)[avail],
					)
				};
			return CommandSlot {
				idx: avail as u8,
				port: self,
				data: tab,
				hdr: hdr,
				event: &self.command_events[avail],
				};
		}
	}
}
impl ::core::ops::Drop for Port
{
	fn drop(&mut self)
	{
		*self.volume.lock() = None;
		//assert!( self.interface_active == false );
	}
}

struct CommandSlot<'a> {
	idx: u8,
	port: &'a Port,
	pub data: &'a mut hw::CmdTable,
	pub hdr: &'a mut hw::CmdHeader,
	pub event: &'a ::kernel::sync::EventChannel,
}
impl<'a> CommandSlot<'a>
{
	// UNSAFE: Caller must ensure that memory pointed to by the `data` table stays valid until the command is complete
	pub unsafe fn start(&self)
	{
		//log_trace!("{} - start(idx={})", self.port, self.idx);
		let mask = 1 << self.idx as usize;
		self.port.regs().write(hw::REG_PxSACT, mask);
		self.port.regs().write(hw::REG_PxCI, mask);
	}

	/// Wait for a command to complete and returns the number of bytes transferred
	pub fn wait(&self) -> Result<usize, Error>
	{
		self.event.sleep();

		let regs = self.port.regs();
		let active = regs.read(hw::REG_PxCI);
		let tfd = regs.read(hw::REG_PxTFD);

		let mask = 1 << self.idx;
		if regs.read(hw::REG_PxSERR) != 0 {
			Err( Error::Bus )
		}
		else if tfd & 0x01 != 0 {
			// Errored (ATA)
			if self.hdr.flags & (1 << 5) == 0 {
				Err( Error::Ata {
					sts: tfd as u8,
					err: (tfd >> 8) as u8,
					} )
			}
			// ATAPI error
			else {
				let err = (tfd >> 8) as u8;
				Err( Error::Atapi {
					sense_key: ::storage_scsi::proto::SenseKey::from(err >> 4),
					eom: err & 2 != 0,
					ili: err & 1 != 0,
					})
			}
		}
		else if active & mask == 0 {
			// All good
			Ok( self.hdr.prdbc as usize )
		}
		else {
			panic!("{} - Command {} woken while still active", self.port, self.idx);
		}
	}
}

impl<'a> ::core::ops::Drop for CommandSlot<'a>
{
	fn drop(&mut self)
	{
		let mask = 1 << self.idx;
		let regs = self.port.regs();
		// SAFE: Reading has no effect
		let cur_active = regs.read(hw::REG_PxCI) /* | regs.read(hw::REG_PxSACT) */;
		if cur_active & mask != 0 {
			todo!("CommandSlot::drop - Port {} cmd {} - Still active", self.port.index, self.idx);
		}
		
		// Release into the pool
		self.port.used_commands.fetch_and(!mask, Ordering::Release);
		self.port.used_commands_sem.release();
	}
}

/// "Interface" - A wrapper around a port that is handed to the SCSI or ATA code
struct Interface(*const Port);
unsafe impl Sync for Interface {}
unsafe impl Send for Interface {}
impl Interface
{
	unsafe fn new(port: &Port) -> Interface {
		Interface(port)
	}
	fn port(&self) -> &Port {
		// SAFE: (TODO) Port should not be dropped before this is (due to handle ownership)
		unsafe { &*self.0 }
	}
}

impl ::storage_ata::volume::Interface for Interface
{
	fn name(&self) -> &str { &self.port().name }

	fn ata_identify(&self) -> Result<::storage_ata::AtaIdentifyData, ::storage_ata::volume::Error> {
		match self.port().request_identify(0xEC)
		{
		Ok(v) => Ok(v),
		Err(Error::Ata{err, ..}) => Err(From::from(err)),
		Err(_) => Err(From::from(0)),
		}
	}
	fn dma_lba_28(&self, cmd: u8, count: u8 , addr: u32, data: DataPtr) -> Result<usize,::storage_ata::volume::Error> {
		match self.port().request_ata_lba28(0, cmd, count, addr, data)
		{
		Ok(bc) => Ok( bc / 512 ),
		Err(Error::Ata{err, ..}) => Err(From::from(err)),
		Err(_) => Err(From::from(0)),
		}
	}
	fn dma_lba_48(&self, cmd: u8, count: u16, addr: u64, data: DataPtr) -> Result<usize,::storage_ata::volume::Error> {
		match self.port().request_ata_lba48(0, cmd, count, addr, data)
		{
		Ok(bc) => Ok( bc / 512 ),
		Err(Error::Ata{err, ..}) => Err(From::from(err)),
		Err(_) => Err(From::from(0)),
		}
	}
}

impl ::storage_scsi::ScsiInterface for Interface
{
	fn name(&self) -> &str {
		&self.port().name
	}
	fn send<'a>(&'a self, command: &[u8], data: &'a [u8]) -> storage::AsyncIoResult<'a,()>
	{
		use storage_scsi::proto::SenseKey;
		Box::pin(::core::future::ready(
			match self.port().request_atapi(0, command, DataPtr::Send(data))
			{
			Ok(_) => Ok( () ),
			Err(Error::Atapi { sense_key: SenseKey::NotReady, .. }) => Err(storage::IoError::NoMedium),
			Err(_) => Err(storage::IoError::Unknown(""))
			}))
	}
	fn recv<'a>(&'a self, command: &[u8], data: &'a mut [u8]) -> storage::AsyncIoResult<'a,()>
	{
		use storage_scsi::proto::SenseKey;
		
		Box::pin(::core::future::ready(
			match self.port().request_atapi(0, command, DataPtr::Recv(data))
			{
			Ok(_) => Ok( () ),
			Err(Error::Atapi { sense_key: SenseKey::NotReady, .. }) => Err(storage::IoError::NoMedium),
			Err(_) => Err(storage::IoError::Unknown(""))
			}))
	}
}
