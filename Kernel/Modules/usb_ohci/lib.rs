// "Tifflin" Kernel - OHCI USB driver
// - By John Hodge (Mutabah / thePowersGang)
//
// Modules/usb_ohci/lib.rs
//! Open Host Controller Interface (OHCI) driver
#![no_std]
#![feature(linkage)]	// for module_define!
use kernel::prelude::*;
use kernel::lib::mem::aref::{Aref,ArefBorrow};
use core::sync::atomic::{AtomicU32,AtomicUsize,Ordering};
use core::mem::size_of;
use ::core::convert::TryFrom;

#[macro_use]
extern crate kernel;
extern crate usb_core;

mod hw;
mod pci;

mod int_buffers;

module_define!{usb_ohci, [usb_core], init}

fn init()
{
	static PCI_DRIVER: pci::PciDriver = pci::PciDriver;
	::kernel::device_manager::register_driver(&PCI_DRIVER);
}

struct BusDev
{
	// Just holds the handle
	_host: Aref<HostInner>,
}
struct UsbHost
{
	host: ArefBorrow<HostInner>,
}
const MAX_INT_PERIOD_MS: usize = 16;
struct HostInner
{
	io: IoWrapper,
	#[allow(dead_code)]
	irq_handle: Option<::kernel::irqs::ObjectHandle>,
	hcca_handle: ::kernel::memory::virt::AllocHandle,
	nports: u8,

	control_list_lock: ::kernel::sync::Spinlock<()>,
	bulk_list_lock: ::kernel::sync::Spinlock<()>,

	// - Async support
	waker: kernel::sync::Spinlock<core::task::Waker>,
	port_update: AtomicU32,
	/// Table containing metadata for each of the entries in the interrupt table (in the HCCA)
	int_table_meta: [InterruptSlotMeta; MAX_INT_PERIOD_MS*2 - 1],
	/// Table of TD metadata (for group 0)
	endpoint_metadata_group0: Vec<EndpointMetadata>,
}
struct IoWrapper(::kernel::device_manager::IOBinding);
#[derive(Default)]
struct InterruptSlotMeta
{
	/// Total number of direct and downstream events on this slot
	loading: AtomicUsize,
}

/// Handle/index to an endpoint
struct EndpointId {
	// Group 0 is in the HCCA page (either in the interrupt graph or the buffers)
	group: u8,
	idx: u8
}
impl_fmt! {
	Debug(self, f) for EndpointId {
		write!(f, "EndpointId({}/{})", self.group, self.idx)
	}
}
/// Index into a pool of transfer descriptors
struct TransferDescriptorId {
	// Group 0 is in the tail end of the HCCA
	group: u8,
	idx: u8,
}
impl_fmt! {
	Debug(self, f) for TransferDescriptorId {
		write!(f, "TransferDescriptorId({}/{})", self.group, self.idx)
	}
}
impl TransferDescriptorId {
	fn from_u16(v: u16) -> TransferDescriptorId {
		TransferDescriptorId {
			group: (v >> 8) as u8,
			idx: (v >> 0) as u8,
			}
	}
	fn to_u16(self) -> u16 {
		(self.group as u16) << 8 | (self.idx as u16) << 0
	}

	// NOTE: Expect panics if this descriptor is used for anything
	fn null() -> TransferDescriptorId {
		TransferDescriptorId {
			group: 0,
			idx: 0,
			}
	}
}
#[derive(Default)]
struct EndpointMetadata {
	tail_td: core::sync::atomic::AtomicU16,
}


impl BusDev
{
	fn new_boxed(irq: u32, io: ::kernel::device_manager::IOBinding) -> Result<Box<BusDev>, &'static str>
	{
		Ok(Box::new(BusDev {
			_host: HostInner::new_aref(irq, io)?
			}))
	}
}
impl IoWrapper
{
	unsafe fn write_reg(&self, r: hw::Regs, v: u32) {
		self.0.write_32(r as usize * 4, v);
	}
	fn read_reg(&self, r: hw::Regs) -> u32 {
		// SAFE: All reads are without side effects
		unsafe { self.0.read_32(r as usize * 4) }
	}
}
impl HostInner
{
	fn new_aref(irq: u32, io: ::kernel::device_manager::IOBinding) -> Result<Aref<HostInner>, &'static str>
	{
		let io = IoWrapper(io);
		let revision = io.read_reg(hw::Regs::HcRevision);
		log_notice!("Card {:?} version is {:#x}", io.0, revision);
		
		let fm_interval_val = io.read_reg(hw::Regs::HcFmInterval);
		let frame_interval = fm_interval_val & 0x3FFF;

		
		// Perform a hardware reset (and get controller from the firmware)
		// SAFE: Read is safe
		let hc_control = io.read_reg(hw::Regs::HcControl);
		if hc_control & 0x100 != 0
		{
			// SMM emulation currently controls the host
			// SAFE: No memory access
			unsafe {
				io.write_reg(hw::Regs::HcCommandStatus, hw::HCCMDSTATUS_OCR);
			}
			todo!("Request device from SMM");
		}
		else
		{
			if hc_control & 0xC0 == 0
			{
				// Bus is in UsbReset, wait a bit then switch to UsbOperational
				// TODO: Wait for a period, or just assume that the wait has already happened.
			}
			else if hc_control & 0xC0 == 0x80
			{
				// Device is ready for operation
			}
			else
			{
				// The bus is in UsbSuspend or UsbResume
				todo!("Reset device, not ready yet");
			}
		}

		// Trigger a reset
		// SAFE: No memory addresses in this one.
		unsafe {
			io.write_reg(hw::Regs::HcCommandStatus, hw::HCCMDSTATUS_HCR);
			// TODO: Wait for 10us
			//::kernel::time::wait_busy_microseconds(10);
			// - Restore the HcFmInterval value
			io.write_reg(hw::Regs::HcFmInterval, fm_interval_val);
			// - Set the bus back to UsbOperational
			io.write_reg(hw::Regs::HcControl, io.read_reg(hw::Regs::HcControl) & !0xC0);
			io.write_reg(hw::Regs::HcControl, io.read_reg(hw::Regs::HcControl) | 0x40);
		}


		// Allocate 'HCCA' (Host Controller Communication Area) and a bunch of other structures.
		// - Use the rest of that page as space for the interrupt structures and TDs
		// The following page contains:
		// -  256 byte HCCA
		// -  512 bytes for interupt graph
		// - 1280 bytes for 16+48 endpoints (general use)
		// - 2048 bytes for 64 transfer descriptors (with 16 bytes of metadata)
		let mut handle_hcca = ::kernel::memory::virt::alloc_dma(32, 1, "usb_ohci")?;
		let stop_endpoint_phys;
		// - Fill the interrupt lists
		{
			let r: &mut hw::IntLists = handle_hcca.as_mut(256);
			let mut next_level_phys = ::kernel::memory::virt::get_phys(r);
			// captures: `next_level_phys`
			let mut init_int_ep = |i, cnt, v: &mut hw::Endpoint| {
				use kernel::memory::PAddr;
				if i == 0 {
					next_level_phys += (cnt * size_of::<hw::Endpoint>()) as PAddr;
				}
				v.next_ed = (next_level_phys + (i as PAddr / 2) * size_of::<hw::Endpoint>() as PAddr) as u32;
				v.flags = 1 << 14;
				};
			// NOTE: Max polling interval is 16ms
			assert_eq!( MAX_INT_PERIOD_MS, 16 );
			assert_eq!( size_of::<hw::IntLists>(), MAX_INT_PERIOD_MS*2 * size_of::<hw::Endpoint>() );
			for (i,v) in r.int_16ms.iter_mut().enumerate()
			{
				init_int_ep(i, 16, v);
			}
			for (i,v) in r.int_8ms.iter_mut().enumerate()
			{
				init_int_ep(i, 8, v);
			}
			for (i,v) in r.int_4ms.iter_mut().enumerate()
			{
				init_int_ep(i, 4, v);
			}
			for (i,v) in r.int_2ms.iter_mut().enumerate()
			{
				init_int_ep(i, 2, v);
			}
			init_int_ep(0, 1, &mut r.int_1ms[0]);
			// - Zero `next_ed` indicates the end of the list
			r.stop_endpoint.next_ed = 0;
			r.stop_endpoint.flags = 1 << 14;
			stop_endpoint_phys = ::kernel::memory::virt::get_phys(&r.stop_endpoint) as u32;
		}

		// Fill the HCCA
		{
			let hcca: &mut hw::Hcca = handle_hcca.as_mut(0);
			let int_base = ::kernel::memory::virt::get_phys(hcca) as u32 + 256;
			let int_indexes = [
				0,8,4,12,2,10,6,14,1,9,5,13,3,11,7,15,
				0,8,4,12,2,10,6,14,1,9,5,13,3,11,7,15,
				];
			for (d,&idx) in Iterator::zip(hcca.interrupt_table.iter_mut(), int_indexes.iter())
			{
				*d = int_base + idx * size_of::<hw::Endpoint>() as u32;
			}
			hcca.frame_number = 0;
			hcca.done_head = 0;
		}

		// Prepare register state
		// SAFE: As safe as it can be
		unsafe
		{
			io.write_reg(hw::Regs::HcControlHeadED, stop_endpoint_phys);
			io.write_reg(hw::Regs::HcBulkHeadED, stop_endpoint_phys);
			io.write_reg(hw::Regs::HcHCCA, ::kernel::memory::virt::get_phys(handle_hcca.as_ref::<u8>(0)) as u32);
			// Enable almost all interrupts
			// - 31: Global enable
			// - 30: Ownership Change (disabled)
			// - 6: Root Hub Status Change
			// - 5: Frame Number Overflow (~1s interrupt?)
			// - 4: Unrecoverable error!
			// - 3: Resume Detect
			// - 2: Start of Frame (disabled)
			// - 1: HcDoneHead written
			// - 0: Scheduling Overrun
			let mask = 0xC000_007F;
			let ints = 0x8000_007B;
			io.write_reg(hw::Regs::HcInterruptEnable, ints & mask);
			io.write_reg(hw::Regs::HcInterruptDisable, (!ints) & mask);
			// Turn on all queues
			// - 10: RemoteWakeupEnable (DISABLED)
			// - 9: RemoteWakeupConnected (DISABLED)
			// - 8: InterruptRouting (DISABLED)
			// - 7/6: HostControllerFunctionalState (=10 UsbOperational)
			// - 5: BulkListEnable
			// - 4: ControlListEnable
			// - 3: IsochronousEnable
			// - 2: PeriodicListEnable
			// - 1/0: ControlBulkServiceRatio (= 00 1:1)
			io.write_reg(hw::Regs::HcControl, 0xBC);	// Turn on all queues
			io.write_reg(hw::Regs::HcPeriodicStart, frame_interval * 9 / 10);	// Program the periodic start point (maximum amount of time for non-int/isoch) to 90%
		}
		
		let nports = (io.read_reg(hw::Regs::HcRhDescriptorA) & 0xFF) as u8;
		assert!(nports <= 15, "Too many ports in OHCI");

		let mut inner_aref = Aref::new(HostInner {
			io: io,
			hcca_handle: handle_hcca,
			nports: nports,

			irq_handle: None,	// Filled below, once the allocation is made

			control_list_lock: Default::default(),
			bulk_list_lock: Default::default(),

			port_update: AtomicU32::new(0),
			waker: kernel::sync::Spinlock::new(kernel::futures::null_waker()),

			int_table_meta: Default::default(),
			endpoint_metadata_group0: Vec::from_fn((1024+256) / 16, |_| Default::default()),
			});
		
		// Bind interrupt
		{
			struct RawSend<T: Send>(*const T);
			unsafe impl<T: Send> Send for RawSend<T> {}
			let ret_raw = RawSend(&*inner_aref);
			// SAFE: Pointer _should_ be valid as long as this IRQ binding exists
			Aref::get_mut(&mut inner_aref).unwrap().irq_handle = Some(::kernel::irqs::bind_object(irq, Box::new(move || unsafe { (*ret_raw.0).handle_irq() } )));
		}
		// Populate `port_update` (could dupicate work from the interrupt, but won't miss anything)
		for i in 0 .. nports as usize
		{
			let v = inner_aref.io.read_reg(inner_aref.get_port_reg(i));
			if v & 0x1 != 0 {	// Is there anything connected?
				inner_aref.port_update.fetch_or(1 << i, Ordering::SeqCst);
			}
		}
		// Pretend there was just an interrupt (causes a read of the interrupt status)
		inner_aref.handle_irq();

		::usb_core::register_host(Box::new(UsbHost { host: inner_aref.borrow() }), nports);
		Ok(inner_aref)
	}

	fn handle_irq(&self) -> bool
	{
		let v = self.io.read_reg(hw::Regs::HcInterruptStatus);
		if v != 0
		{
			log_trace!("handle_irq: {:#x}", v);
			
			// SchedulingOverrun
			if v & 0x01 != 0
			{
				log_notice!("USB Scheduling Overrun");
			}
			// WritebackDoneHead
			if v & 0x02 != 0
			{
				// Clear the contents of HccaDoneHead, releasing and completing those TDs
				let mut phys = self.hcca_handle.as_ref::<hw::Hcca>(0).done_head & !0xF;
				while phys != 0
				{
					let td_id = match self.get_general_td_from_phys(phys)
						{
						Some(id) => id,
						None => panic!("WritebackDoneHead: {:#x}", phys),
						};
					log_debug!("WritebackDoneHead - {:?}", td_id);
					let ptr = self.get_general_td_pointer(&td_id);
					phys = ptr.get_next();

					let cc = ptr.read_flags().get_cc();
					if cc != hw::CompletionCode::NoError {
						log_debug!("WritebackDoneHead: {:?} cc={:?}", td_id, cc);
						panic!("Handle non-zero condition code: {:?} cc={:?}", td_id, cc);
					}

					// Read waker out
					let waker = ptr.take_waker();

					// Mark as complete (and release if FLAG_AUTOFREE)
					if ptr.mark_complete()
					{
						ptr.mark_free();
					}

					// Poke the waker
					waker.wake();
				}
				log_debug!("WritebackDoneHead - {:#x}", phys);
			}
			// StartofFrame (disabled)
			if v & 0x04 != 0
			{

			}
			// ResumeDetected
			if v & 0x08 != 0
			{
				// A device is asking for a resume?
			}
			// UnrecoverableError
			if v & 0x10 != 0
			{
				log_error!("Unrecoverable error!");
			}
			// FrameNumberOverflow
			if v & 0x20 != 0
			{
				// Frame number has reached 2^15
			}
			// RootHubStatusChange
			if v & 0x40 != 0
			{
				// A change to any of the root hub registers
				for i in 0 .. self.nports
				{
					let v = self.io.read_reg(self.get_port_reg(i as usize));
					if v & 0xFFFF_0000 != 0 {
						log_debug!("Status change on port {} = {:#x}", i, v);

						// - async
						self.port_update.fetch_or(1 << i, Ordering::SeqCst);
					}
				}
				if self.port_update.load(Ordering::SeqCst) != 0
				{
					self.waker.lock().wake_by_ref();
				}
			}

			// SAFE: Write clear, no memory unsafety
			unsafe { self.io.write_reg(hw::Regs::HcInterruptStatus, v) };

			true
		}
		else
		{
			false
		}
	}

	/// Allocate a new endpoint
	fn allocate_endpoint(&self, flags: u32) -> EndpointId
	{
		let ep_id = (|| {
			// 1. Iterate all group 0 endpoints and look for one not marked as allocated
			// - Free pool starts at 256 + 512 (HCCA + interrupts)
			for i in (256 + 512) / 16 .. 2048 / 16
			{
				let ep_id = EndpointId { group: 0, idx: i as u8 };
				let ptr = self.get_ed_pointer(&ep_id);
				// SAFE: Pointer is valid (we just got it from get_ed_pointer)
				let flags_atomic = unsafe { &*hw::Endpoint::atomic_flags(ptr) };
				let fv = flags_atomic.load(Ordering::SeqCst);
				if fv & hw::Endpoint::FLAG_ALLOC == 0 {
					if flags_atomic.compare_exchange(fv, flags | hw::Endpoint::FLAG_ALLOC, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
						return ep_id;
					}
				}
			}
			// 2. Look through already-existing endpoint pages
			todo!("allocate_endpoint: flags={:#x} (hcca page full)", flags)
			})();
		log_debug!("allocate_endpoint(flags={:#x}): ptr={:#x}", flags, kernel::memory::virt::get_phys(self.get_ed_pointer(&ep_id)));
		// - Populate metadata and initialise them
		let meta = self.get_endpoint_meta(&ep_id);
		let new_tail = self.allocate_td();
		let mut h = self.get_ed_locked(&ep_id);
		// SAFE: Locked
		unsafe {
			let tp = kernel::memory::virt::get_phys( self.get_general_td_pointer(&new_tail) ) as u32;
			h.set_head_ptr( tp );
			h.set_tail_ptr( tp );
			h.set_next_ed(0);
		}
		meta.tail_td.store( new_tail.to_u16(), Ordering::SeqCst );

		ep_id
	}
	/// Obtain a pointer to the specified endpoint descriptor
	// NOTE: Returns a raw pointer because it's possibly being mutated
	fn get_ed_pointer(&self, id: &EndpointId) -> *const hw::Endpoint {
		if id.group == 0 {
			let ofs = (id.idx as usize) * size_of::<hw::Endpoint>();
			assert!(ofs >= 256);
			assert!(ofs < 2048);
			self.hcca_handle.as_ref(ofs)
		}
		else {
			todo!("get_ed_pointer: Alternate pools for endpoint descriptors");
		}
	}

	/// Obtain a lock handle for a given endpoint descriptor
	fn get_ed_locked(&self, id: &EndpointId) -> LockedEndpoint {
		// SAFE: Pointer is valid
		unsafe {
			LockedEndpoint::new(self.get_ed_pointer(id) as *mut hw::Endpoint)
		}
	}



	fn get_general_td_from_phys(&self, addr: u32) -> Option<TransferDescriptorId> {
		let hcca_page = ::kernel::memory::virt::get_phys(self.hcca_handle.as_ref::<()>(0)) as u32;
		if addr & !0xFFF == hcca_page {
			let ofs = (addr - hcca_page) as usize;
			assert!(ofs % size_of::<hw::GeneralTD>() == 0);
			return Some(TransferDescriptorId { group: 0, idx: (ofs / size_of::<hw::GeneralTD>()) as u8 });
		}
		None
	}
	fn get_general_td_pointer(&self, id: &TransferDescriptorId) -> &hw::GeneralTD {
		if id.group == 0 {
			let ofs = (id.idx as usize) * size_of::<hw::GeneralTD>();
			assert!(ofs >= 2048, "TD {:?} invalid", id);
			assert!(ofs < 4096);
			// SAFE: Aligned, in range, and that's what's in this region
			unsafe { &*(self.hcca_handle.as_ref::<u8>(ofs) as *const _ as *const hw::GeneralTD) }
		}
		else {
			todo!("get_general_td_pointer: Alternate pools for transfer descriptors");
		}
	}

	/// Obtain metadata for the specified endpoint
	fn get_endpoint_meta(&self, id: &EndpointId) -> &EndpointMetadata {
		// TODO: Find a place where the metadata can be stored
		if id.group == 0 {
			&self.endpoint_metadata_group0[id.idx as usize]
		}
		else {
			todo!("get_endpoint_meta({:?})", id);
		}
	}

	/// Register an interrupt endpoint
	fn register_interrupt_ed(&self, period_pow_2: usize, flags: u32) -> EndpointId
	{
		// 1. Find a low-load slot of this period
		let (start,len) = 
			match period_pow_2
			{
			4 => (0, 16),
			3 => (16, 8),
			2 => (16+8, 4),
			1 => (16+8+4, 2),
			0 => (16+8+4+2, 1),
			_ => (0, 16),
			};
		let meta = &self.int_table_meta[start..][..len];
		let min_slot_idx = (0 .. len).min_by_key(|&i| meta[i].loading.load(Ordering::SeqCst)).unwrap();
		let placeholder_ed_id = EndpointId {
			group: 0,
			idx: (256 / size_of::<hw::Endpoint>() + start + min_slot_idx) as u8,
			};
		// Increment loading of this slot and all down-stream slots
		for idx in UpstreamIntSlots(start + min_slot_idx)
		{
			self.int_table_meta[idx].loading.fetch_add(1, Ordering::SeqCst);
		}
		let mut placeholder_ed = self.get_ed_locked(&placeholder_ed_id);

		// 2. Check if the placeholder is in use
		if placeholder_ed.flags() & (1 << 14) == 0 {
			// - If it is, allocate a new endpoint descriptor and put it after the placeholder
			let new_ed_id = self.allocate_endpoint(flags);

			// SAFE: Ordering ensures consistency, writing valid addreses
			unsafe {
				let mut new_ed = self.get_ed_locked(&new_ed_id);
				new_ed.set_next_ed( placeholder_ed.next_ed() );
				placeholder_ed.set_next_ed( new_ed.get_phys() );
			}

			new_ed_id
		}
		else {
			// - Otherwise use the placeholder
			placeholder_ed.set_flags(flags);

			let meta = self.get_endpoint_meta(&placeholder_ed_id);
			let new_tail = self.allocate_td();
			// SAFE: Locked
			unsafe {
				let tp = kernel::memory::virt::get_phys( self.get_general_td_pointer(&new_tail) ) as u32;
				placeholder_ed.set_head_ptr( tp );
				placeholder_ed.set_tail_ptr( tp );
				placeholder_ed.set_next_ed(0);
			}
			meta.tail_td.store( new_tail.to_u16(), Ordering::SeqCst );

			placeholder_ed_id
		}
	}
	/// Register a general-purpose endpoint descriptor and add it to the control queue
	fn register_control_ed(&self, flags: u32) -> EndpointId
	{
		let ep = self.allocate_endpoint(flags);

		// SAFE: Pointer valid, register access controlled
		unsafe {
			let ptr = self.get_ed_pointer(&ep) as *mut hw::Endpoint;
			let paddr = ::kernel::memory::virt::get_phys(ptr) as u32;

			// Lock list
			let _lh = self.control_list_lock.lock();
			// Get existing head pointer, store in newly created ED, update register
			let existing = self.io.read_reg(hw::Regs::HcControlHeadED);
			(*ptr).next_ed = existing;
			self.io.write_reg(hw::Regs::HcControlHeadED, paddr);
		}
		ep
	}
	/// Register a general-purpose endpoint descriptor and add it to the bulk queue
	fn register_bulk_ed(&self, flags: u32) -> EndpointId
	{
		let ep = self.allocate_endpoint(flags);

		// SAFE: Pointer valid, register access controlled
		unsafe {
			let ptr = self.get_ed_pointer(&ep) as *mut hw::Endpoint;
			let paddr = ::kernel::memory::virt::get_phys(ptr) as u32;

			// Lock list
			let _lh = self.bulk_list_lock.lock();
			// Get existing head pointer, store in newly created ED, update register
			let existing = self.io.read_reg(hw::Regs::HcBulkHeadED);
			(*ptr).next_ed = existing;
			self.io.write_reg(hw::Regs::HcBulkHeadED, paddr);
		}
		ep
	}

	/// Allocate a new TD
	fn allocate_td(&self) -> TransferDescriptorId
	{
		// Iterate over all avaliable pools
		const SIZE: usize = size_of::<hw::GeneralTD>();
		for i in (2048 / SIZE) .. (4096 / SIZE)
		{
			let rv = TransferDescriptorId { group: 0, idx: i as u8 };
			if self.get_general_td_pointer(&rv).maybe_alloc()
			{
				//log_debug!("allocate_td: group 0, idx {}", i);
				return rv;
			}
		}
		todo!("allocate_td - alternate pools, main is exhausted");
	}
	unsafe fn push_td(&self, ep: &EndpointId, flags: u32, first_byte: u32, last_byte: u32, waker: ::core::task::Waker) -> TransferDescriptorId
	{
		//log_debug!("push_td({:?}, {:#x}, {:#x}-{:#x})", ep, flags, first_byte, last_byte);
		// 1. Allocate a new transfer descriptor (to be used as the new tail)
		let new_tail_td = self.allocate_td();
		let new_tail_phys = ::kernel::memory::virt::get_phys( self.get_general_td_pointer(&new_tail_td) ) as u32;
		// 2. Lock the endpoint (makes sure that there's no contention software-side)
		// TODO: Could the metadata be locked to the endpoint handle?
		let mut ed = self.get_ed_locked(ep);
		let epm = self.get_endpoint_meta(ep);
		// - Update the tail in metadata (swap for the newly allocated one)
		let td_handle = TransferDescriptorId::from_u16( epm.tail_td.swap(new_tail_td.to_u16(), Ordering::SeqCst) );
		log_trace!("push_td({:?}, {:#x}, {:#x}-{:#x}): {:?}", ep, flags, first_byte, last_byte, td_handle);
		// - Obtain pointer to the old tail
		let td_ptr = self.get_general_td_pointer(&td_handle);
		// - Fill the old tail with our data (and the new tail paddr)
		hw::GeneralTD::init(td_ptr as *const _ as *mut _, flags, first_byte, last_byte, new_tail_phys, waker);
		// - Update the tail pointer
		ed.set_tail_ptr( new_tail_phys );

		td_handle
	}
	pub fn stop_td(&self, td: &TransferDescriptorId)
	{
		todo!("stop_td({:?})", td);
	}
	pub fn release_td(&self, td: TransferDescriptorId)
	{
		(*self.get_general_td_pointer(&td)).mark_free();
	}

	/// Kick the controller and make it run the control list
	fn kick_control(&self)
	{
		// SAFE: No memory impact
		unsafe {
			self.io.write_reg(hw::Regs::HcCommandStatus, hw::HCCMDSTATUS_CLF);
		}
	}
	/// Kick the controller and make it run the bulk list
	fn kick_bulk(&self)
	{
		// SAFE: No memory impact
		unsafe {
			self.io.write_reg(hw::Regs::HcCommandStatus, hw::HCCMDSTATUS_BLF);
		}
	}

	fn td_update_waker(&self, td: &TransferDescriptorId, waker: &::core::task::Waker)
	{
		self.get_general_td_pointer(td).update_waker(waker)
	}
	fn td_complete(&self, td: &TransferDescriptorId) -> Option<usize> {
		self.get_general_td_pointer(td).is_complete()
	}

	// Get a handle for a DMA output
	fn get_dma_todev(&self, p: &[u8]) -> (Option<::kernel::memory::virt::AllocHandle>, u32, u32)
	{
		log_debug!("get_dma_todev({:p})", p);
		if p.len() == 0 {
			return (None, 0 as u32, 0 as u32);
		}
		let start_phys = ::kernel::memory::virt::get_phys(p.as_ptr());
		let last_phys = ::kernel::memory::virt::get_phys(&p[p.len()-1]);

		match (u32::try_from(start_phys), u32::try_from(last_phys))
		{
		(Ok(start_phys), Ok(last_phys)) => 
			if start_phys & !0xFFF != last_phys & !0xFFF && (0x1000 - (start_phys & 0xFFF) + last_phys & 0xFFF) as usize != p.len() {
				// The buffer spans more than two pages, bounce
			}
			else {
				// Good
				return (None, start_phys, last_phys);
			},
		_ => {
			// An address is more than 32-bits, bounce
			}
		}
		todo!("Bounce buffer - long lifetime");
	}
	// Get a handle for a DMA input
	fn get_dma_fromdev<'a>(&self, p: &'a mut [u8]) -> (Option<::kernel::memory::virt::AllocHandle>, u32, u32)
	{
		let start_phys = ::kernel::memory::virt::get_phys(p.as_ptr());
		let last_phys = ::kernel::memory::virt::get_phys(&p[p.len()-1]);
		match (u32::try_from(start_phys), u32::try_from(last_phys))
		{
		(Ok(start_phys), Ok(last_phys)) => 
			if start_phys & !0xFFF != last_phys & !0xFFF && (0x1000 - (start_phys & 0xFFF) + last_phys & 0xFFF) as usize != p.len() {
				// The buffer spans more than two pages, bounce
			}
			else {
				// Good
				return (None, start_phys, last_phys);
			}
		_ => {
			// An address is more than 32-bits, bounce
			}
		}
		todo!("Bounce buffer for read");
	}

	fn get_port_reg(&self, port: usize) -> hw::Regs
	{
		assert!(port < 16);
		assert!(port < self.nports as usize);
		// SAFE: Bounds are checked to fit within the alowable range for the enum
		unsafe { ::core::mem::transmute(hw::Regs::HcRhPortStatus0 as usize + port) }
	}
}


/// Lock handle on a `hw::Endpoint`
struct LockedEndpoint<'a> {
	_lt: ::core::marker::PhantomData<&'a HostInner>,
	ptr: *mut hw::Endpoint,
	_held_interrupts: kernel::arch::sync::HeldInterrupts,
}
impl<'a> LockedEndpoint<'a>
{
	// UNSAFE: Ensure pointer is valid
	unsafe fn new(ptr: *mut hw::Endpoint) -> LockedEndpoint<'a> {
		// TODO: Lock by:
		// - Blocking interrupts
		let held_interrupts = kernel::arch::sync::hold_interrupts();
		// - doing a CAS loop on bit 31 of flags
		let flags_atomic = &*hw::Endpoint::atomic_flags(ptr);
		loop {
			let v = flags_atomic.load(Ordering::Acquire) & !hw::Endpoint::FLAG_LOCKED;

			if flags_atomic.compare_exchange(v, v | hw::Endpoint::FLAG_LOCKED, Ordering::Acquire, Ordering::Relaxed).is_ok() {
				break;
			}
		}
		LockedEndpoint {
			_lt: core::marker::PhantomData,
			ptr: ptr,
			_held_interrupts: held_interrupts,
			}
	}

	/// Obtain the physical address of this endpoint descriptor
	fn get_phys(&self) -> u32 {
		kernel::memory::virt::get_phys(self.ptr) as u32
	}

	// SAFE: Read-only, locked
	pub fn flags   (&self) -> u32 { unsafe { (*self.ptr).flags    } }
	// SAFE: Read-only, locked
	//pub fn tail_ptr(&self) -> u32 { unsafe { (*self.ptr).tail_ptr } }
	// NOTE: The controller can write to this value, so use read_volatile
	// SAFE: Read-only, locked
	//pub fn head_ptr(&self) -> u32 { unsafe { core::ptr::read_volatile(&(*self.ptr).head_ptr) } }
	// SAFE: Read-only, locked
	pub fn next_ed (&self) -> u32 { unsafe { (*self.ptr).next_ed  } }

	// Safe field, so not unsafe to call
	pub fn set_flags(&mut self, v: u32) {
		// SAFE: Value cannot cause unsafety on its own, locked
		unsafe {
			(*self.ptr).flags = v | (1 << 31);	// maintain the lock bit
		}
	}
	/// UNSAFE: Value must be a valid physical address
	unsafe fn set_tail_ptr(&mut self, v: u32) {
		core::ptr::write_volatile( &mut (*self.ptr).tail_ptr, v );
	}
	/// UNSAFE: Value must be a valid physical address
	unsafe fn set_head_ptr(&mut self, v: u32) {
		// TODO: The controller writes to this field too
		core::ptr::write_volatile( &mut (*self.ptr).head_ptr, v );
	}
	/// UNSAFE: Value must be a valid physical address
	unsafe fn set_next_ed(&mut self, v: u32) {
		core::ptr::write_volatile( &mut (*self.ptr).next_ed, v );
	}
}
impl<'a> core::ops::Drop for LockedEndpoint<'a> {
	fn drop(&mut self) {
		// Write back `flags` ensuring that the lock bit (31) is clear
		// SAFE: Atomic accesses, valid pointer
		unsafe {
			let new_flags = self.flags() & !hw::Endpoint::FLAG_LOCKED;
			(*hw::Endpoint::atomic_flags(self.ptr)).store( new_flags, Ordering::Release );
		}
		// Interrupt hold released on inner drop
	}
}


/// Iterator over the "upstream" slots for a given interrupt slot
/// I.e. the slots that would have increased loading if an item was added to this slot
struct UpstreamIntSlots(usize);
impl Iterator for UpstreamIntSlots
{
	type Item = usize;
	fn next(&mut self) -> Option<usize>
	{
		if self.0 == MAX_INT_PERIOD_MS*2 - 1 {
			None
		}
		else {
			let cur = self.0;
			let (mut base, mut size) = (0,MAX_INT_PERIOD_MS);
			while size > 1 {
				if cur-base < size {
					self.0 = base + size + (self.0 - base) / 2;
					break;
				}
				base += size;
				size /= 2;
			}
			if size == 1 {
				assert_eq!(cur, base);
				assert_eq!(cur, MAX_INT_PERIOD_MS*2 - 2);
				self.0 = MAX_INT_PERIOD_MS*2 - 1;
			}
			Some(cur)
		}
	}
}

use ::usb_core::host::{self, EndpointAddr, PortFeature, Handle};
use ::usb_core::host::{InterruptEndpoint, IsochEndpoint, ControlEndpoint};

impl ::usb_core::host::HostController for UsbHost
{
	fn init_interrupt(&self, endpoint: EndpointAddr, period_ms: usize, max_packet_size: usize) -> Handle<dyn InterruptEndpoint> {
		Handle::new(InterruptEndpointHandle::new(self.host.reborrow(), endpoint, period_ms, max_packet_size))
			.or_else(|v| Handle::new(Box::new(v)))
			.ok().expect("Box doesn't fit in alloc")
	}
	fn init_isoch(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn IsochEndpoint> {
		todo!("init_isoch({:?}, max_packet_size={})", endpoint, max_packet_size);
	}
	fn init_control(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn ControlEndpoint> {
		// Allocate an endpoint
		let ptr = self.host.register_control_ed(
			  (endpoint.dev_addr() as u32 & 0x7F) << 0
			| ((endpoint.endpt() & 0xF) << 7) as u32
			| (0b00 << 11)	// Direction - Use TD
			| (0b0 << 13)	// Speed (TODO)
			| (0b0 << 14)	// Skip - clear
			| (0b0 << 15)	// Format - 0=control/bulk/int
			| (max_packet_size as u32 & 0xFFFF) << 16
			);
		log_debug!("init_control({:?}): {:?}", endpoint, ptr);
		Handle::new(ControlEndpointHandle {
			controller: self.host.reborrow(),
			id: ptr,
			}).ok().unwrap()
	}
	fn init_bulk_out(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::BulkEndpointOut> {
		let ptr = self.host.register_bulk_ed(
			  (endpoint.dev_addr() as u32 & 0x7F) << 0
			| (endpoint.endpt() as u32 & 0xF) << 7
			| (0b01 << 11)	// Direction - OUT
			| (0b0 << 13)	// Speed (TODO)
			| (0b0 << 14)	// Skip - clear
			| (0b0 << 15)	// Format - 0=control/bulk/int
			| (max_packet_size as u32 & 0xFFFF) << 16
			);
		log_debug!("init_bulk_out({:?}): {:?}", endpoint, ptr);
		Handle::new(BulkEndpointOut {
			controller: self.host.reborrow(),
			id: ptr,
			}).ok().unwrap()
	}
	fn init_bulk_in(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn host::BulkEndpointIn> {
		let ptr = self.host.register_bulk_ed(
			  (endpoint.dev_addr() as u32 & 0x7F) << 0
			| (endpoint.endpt() as u32 & 0xF) << 7
			| (0b10 << 11)	// Direction - IN
			| (0b0 << 13)	// Speed (TODO)
			| (0b0 << 14)	// Skip - clear
			| (0b0 << 15)	// Format - 0=control/bulk/int
			| (max_packet_size as u32 & 0xFFFF) << 16
			);
		log_debug!("init_bulk_int({:?}): {:?}", endpoint, ptr);
		Handle::new(BulkEndpointIn {
			controller: self.host.reborrow(),
			id: ptr,
			}).ok().unwrap()
	}


	// Root hub maintainence
	fn set_port_feature(&self, port: usize, feature: PortFeature) {
		log_trace!("set_port_feature({}, {:?})", port, feature);
		let r = self.host.get_port_reg(port);
		// All bits only set on write (some clear on write, but those aren't handled here.
		let v = match feature
			{
			PortFeature::Enable    => 0x0002,
			PortFeature::Suspend   => 0x0004,
			PortFeature::Reset     => 0x0010,
			PortFeature::Power     => 0x0100,
			PortFeature::Test      => return,	// not supported
			PortFeature::Indicator => return,	// not supported
			_ => return,
			};
		// SAFE: Can't cause memory unsafety
		unsafe {
			self.host.io.write_reg(r, v);
		}
	}
	fn clear_port_feature(&self, port: usize, feature: PortFeature) {
		log_trace!("clear_port_feature({}, {:?})", port, feature);
		let r = self.host.get_port_reg(port);
		// All bits only set on write (some clear on write, but those aren't handled here.
		let v = match feature
			{
			PortFeature::Enable  => 0x0001,	// bit 0 - ConnectionStatus / ClearEnableStatus
			PortFeature::Suspend => 0x0008,	// bit 3 - PortOverCurrentIndicator / ClearSuspendStatus
			PortFeature::Reset   => return,	// - No clear
			PortFeature::Power   => 0x0200,	// bit ? - LowSpeedDeviceAttached / ClearPortPower
			PortFeature::CConnection => 0x01_0000,
			PortFeature::CEnable     => 0x02_0000,
			PortFeature::CSuspend    => 0x04_0000,
			PortFeature::COverCurrent=> 0x08_0000,
			PortFeature::CReset      => 0x10_0000,
			_ => return,
			};
		// SAFE: Can't cause memory unsafety
		unsafe {
			self.host.io.write_reg(r, v);
		}
	}
	fn get_port_feature(&self, port: usize, feature: PortFeature) -> bool {
		log_trace!("get_port_feature({}, {:?})", port, feature);
		let r = self.host.get_port_reg(port);
		let v = self.host.io.read_reg(r);
		let mask = match feature
			{
			PortFeature::Connection  => 0x0001,
			PortFeature::Enable      => 0x0002,
			PortFeature::Suspend     => 0x0004,
			PortFeature::OverCurrent => 0x0008,
			PortFeature::Reset       => 0x0010,
			PortFeature::Power       => 0x0100,
			PortFeature::LowSpeed    => 0x0200,
			PortFeature::CConnection => 0x01_0000,
			PortFeature::CEnable     => 0x02_0000,
			PortFeature::CSuspend    => 0x04_0000,
			PortFeature::COverCurrent=> 0x08_0000,
			PortFeature::CReset      => 0x10_0000,
			PortFeature::Test        => return false,
			PortFeature::Indicator   => return false,
			};
		v & mask != 0
	}

	fn async_wait_root(&self) -> usb_core::host::AsyncWaitRoot {
		struct AsyncWaitRoot {
			host: ArefBorrow<HostInner>,
		}
		impl core::future::Future for AsyncWaitRoot {
			type Output = usize;
			fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context) -> core::task::Poll<Self::Output> {
				let v = self.host.port_update.load(Ordering::SeqCst);
				log_debug!("UsbHost::AsyncWaitRoot::poll: v = {:#x}", v);
				if v != 0
				{
					for i in 0 .. self.host.nports as usize
					{
						let bit = 1 << i;
						if v & bit != 0 {
							self.host.port_update.fetch_and(!bit, Ordering::SeqCst);
							return core::task::Poll::Ready(i);
						}
					}
				}
				*self.host.waker.lock() = cx.waker().clone();
				core::task::Poll::Pending
			}
		}
		usb_core::host::AsyncWaitRoot::new(AsyncWaitRoot {
			host: self.host.reborrow(),
			}).ok().expect("Over-size task in")
	}
}
struct ControlEndpointHandle {
	controller: ArefBorrow<HostInner>,
	id: EndpointId,
}
impl host::ControlEndpoint for ControlEndpointHandle
{
	fn out_only<'a>(&'a self, setup_data: &'a [u8], out_data: &'a [u8]) -> host::AsyncWaitIo<'a, usize>
	{
		enum FutureState<'a> {
			Init {
				setup_data: &'a [u8],
				out_data: &'a [u8],
				},
			Started {
				_bb_setup: Option<::kernel::memory::virt::AllocHandle>,
				_bb_data: Option<::kernel::memory::virt::AllocHandle>,
				out_data_len: usize,
				td_setup: TransferDescriptorId,
				td_data: TransferDescriptorId,
				td_status: TransferDescriptorId,
				},
			Complete,
		}
		struct Future<'a> {
			self_: &'a ControlEndpointHandle,
			state: FutureState<'a>,
		}
		impl<'a> core::future::Future for Future<'a> {
			type Output = usize;
			fn poll(mut self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context) -> core::task::Poll<Self::Output> {
				let parent = self.self_;
				match self.state
				{
				FutureState::Init { setup_data, out_data } => {
					log_debug!("out_only - init");
					// Get (potentially bounced) data handles
					let (setup_buf, setup_first_phys, setup_last_phys) = parent.controller.get_dma_todev(setup_data);
					let (out_buf, out_first_phys, out_last_phys) = parent.controller.get_dma_todev(out_data);

					// TODO: This isn't 100% safe, as the future _could_ be leaked before completion
					// SAFE: Requires that the future isn't leaked
					unsafe {
						use kernel::futures::null_waker;
						let td_setup = parent.controller.push_td( &parent.id, hw::GeneralTdFlags::new_setup().autofree().no_int().into(), setup_first_phys, setup_last_phys, null_waker() );
						let td_data  = parent.controller.push_td( &parent.id, hw::GeneralTdFlags::new_out().no_int().into(), out_first_phys, out_last_phys, null_waker() );
						let td_status= parent.controller.push_td( &parent.id, hw::GeneralTdFlags::new_in().into(), 0, 0, cx.waker().clone() );
						parent.controller.kick_control();
						self.state = FutureState::Started {
							_bb_setup: setup_buf,
							_bb_data: out_buf,
							out_data_len: out_data.len(),
							td_setup,
							td_data,
							td_status,
							};
					}
					core::task::Poll::Pending
					},
				FutureState::Started { ref td_status, .. } =>
					if parent.controller.td_complete(td_status).is_some()
					{
						log_debug!("out_only - Started -> complete");
						match core::mem::replace(&mut self.state, FutureState::Complete)
						{
						FutureState::Started { td_data, td_status, out_data_len, .. } => {
							let spare_size = parent.controller.td_complete(&td_data).unwrap();
							log_debug!("out_only - out_data_len={}, spare_size={}", out_data_len, spare_size);
							parent.controller.release_td(td_data);
							parent.controller.release_td(td_status);
							core::task::Poll::Ready(out_data_len)
							},
						_ => panic!(),
						}
					}
					else
					{
						log_debug!("out_only - Started -> pending");
						parent.controller.td_update_waker(td_status, cx.waker());
						core::task::Poll::Pending
					},
				FutureState::Complete => panic!("Completed future polled"),
				}
			}
		}
		impl<'a> core::ops::Drop for Future<'a> {
			fn drop(&mut self)
			{
				match self.state
				{
				FutureState::Init { .. } => {},
				FutureState::Started { ref td_setup, ref td_data, .. } => {
					// TODO: Force termination of the transfers
					self.self_.controller.stop_td(td_setup);
					self.self_.controller.stop_td(td_data);
					}
				FutureState::Complete => {},
				}
			}
		}
		host::AsyncWaitIo::new(Future {
			self_: self,
			state: FutureState::Init {
				setup_data,
				out_data,
				}
			})
			.or_else(|v| host::AsyncWaitIo::new(Box::new(v)))
			.ok().expect("Box doesn't fit in alloc")
	}
	fn in_only<'a>(&'a self, setup_data: &'a [u8], in_data: &'a mut [u8]) -> ::usb_core::host::AsyncWaitIo<'a, usize>
	{
		enum FutureState<'a> {
			Init {
				setup_data: &'a [u8],
				in_data: &'a mut [u8],
				},
			Started {
				_bb_setup: Option<::kernel::memory::virt::AllocHandle>,
				bb_data: Option<::kernel::memory::virt::AllocHandle>,
				in_data: &'a mut [u8],
				td_setup: TransferDescriptorId,
				td_data: TransferDescriptorId,
				td_status: TransferDescriptorId,
				},
			Complete,
		}
		struct Future<'a> {
			self_: &'a ControlEndpointHandle,
			state: FutureState<'a>,
		}
		impl<'a> core::future::Future for Future<'a> {
			type Output = usize;
			fn poll(mut self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context) -> core::task::Poll<usize> {
				let parent = self.self_;
				match self.state
				{
				FutureState::Init { setup_data, ref mut in_data } => {
					log_debug!("in_only - init");
					let in_data = ::core::mem::replace(in_data, &mut []);
					// Get (potentially bounced) data handles
					let (setup_buf, setup_first_phys, setup_last_phys) = parent.controller.get_dma_todev(setup_data);
					let (in_buf, in_first_phys, in_last_phys) = parent.controller.get_dma_fromdev(in_data);

					// TODO: This isn't 100% safe, as the future _could_ be leaked before completion
					// SAFE: Requires that the future isn't leaked
					unsafe {
						use kernel::futures::null_waker;
						let td_setup = parent.controller.push_td( &parent.id, hw::GeneralTdFlags::new_setup().autofree().no_int().into(), setup_first_phys, setup_last_phys, null_waker() );
						let td_data  = parent.controller.push_td( &parent.id, hw::GeneralTdFlags::new_in().no_int().rounding().into(), in_first_phys, in_last_phys, null_waker() );
						let td_status= parent.controller.push_td( &parent.id, hw::GeneralTdFlags::new_out().into(), 0, 0, cx.waker().clone() );
						parent.controller.kick_control();
						self.state = FutureState::Started {
							_bb_setup: setup_buf,
							bb_data: in_buf,
							in_data: in_data,
							td_setup,
							td_data,
							td_status,
							};
					}
					core::task::Poll::Pending
					},
				FutureState::Started { ref td_status, .. } =>
					if parent.controller.td_complete(td_status).is_some()
					{
						match core::mem::replace(&mut self.state, FutureState::Complete)
						{
						FutureState::Started { td_data, td_status, bb_data, in_data, .. } => {
							let rem_size = parent.controller.td_complete(&td_data).unwrap();
							assert!(rem_size <= in_data.len(), "{} <= {}", rem_size, in_data.len());
							let read_len = if rem_size == in_data.len() { in_data.len() } else { in_data.len() - rem_size };
							log_debug!("in_only - completed {} (read {})", rem_size, read_len);
							if let Some(r) = bb_data {
								in_data.copy_from_slice( r.as_slice(0, in_data.len()) );
							}

							parent.controller.release_td(td_data);
							parent.controller.release_td(td_status);
							core::task::Poll::Ready(read_len)
							},
						_ => panic!(""),
						}
					}
					else
					{
						log_debug!("in_only - pending");
						parent.controller.td_update_waker(td_status, cx.waker());
						core::task::Poll::Pending
					},
				FutureState::Complete => panic!("Complete"),
				}
			}
		}
		impl<'a> core::ops::Drop for Future<'a> {
			fn drop(&mut self)
			{
				match self.state
				{
				FutureState::Init { .. } => {},
				FutureState::Started { ref td_setup, ref td_data, .. } => {
					// TODO: Force termination of the transfers
					self.self_.controller.stop_td(td_setup);
					self.self_.controller.stop_td(td_data);
					}
				FutureState::Complete => {},
				}
			}
		}
		host::AsyncWaitIo::new(Future {
			self_: self,
			state: FutureState::Init {
				setup_data,
				in_data,
				}
			})
			.or_else(|v| host::AsyncWaitIo::new(Box::new(v)))
			.ok().expect("Box doesn't fit in alloc")
	}
}

struct InterruptEndpointHandle {
	controller: ArefBorrow<HostInner>,
	id: EndpointId,
	current_td: ::kernel::sync::Spinlock< Option< (TransferDescriptorId, int_buffers::FillingHandle, )> >,
	buffers: int_buffers::InterruptBuffers,
}
impl InterruptEndpointHandle
{
	fn new(host: ArefBorrow<HostInner>, endpoint: EndpointAddr, period_ms: usize, max_packet_size: usize) -> Self
	{
		// NOTE: This rounds down (so 3 = 2^1)
		let period_pow_2 = if period_ms == 0 { 0 } else { 32-1 - (period_ms as u32).leading_zeros()};
		let ptr = host.register_interrupt_ed(period_pow_2 as usize,
			  (endpoint.dev_addr() & 0x7F) as u32
			| ((endpoint.endpt() & 0xF) << 7) as u32
			| (0b00 << 11)	// Direction - Use TD
			| (0b0 << 13)	// Speed (TODO)
			| (0b0 << 14)	// Skip - clear
			| (0b0 << 15)	// Format - 0=control/bulk/int
			// TODO: max packet size?
			);
		log_debug!("init_interrupt({:?}): {:?}", endpoint, ptr);
		// Allocate a pair of buffers (in DMA memory) of `max_packet_size`, use double buffering for them
		// NOTE: Lazy option: Allocate a whole page (a pool would be better/more efficient)
		// NOTE: Don't add TDs until `wait` is called, ensures that the first packet after wait is for the wait
		InterruptEndpointHandle {
			controller: host,
			id: ptr,
			buffers: int_buffers::InterruptBuffers::new(max_packet_size),
			current_td: Default::default(),
			}
	}

	// UNSAFE: Must ensure that the buffer TD is stopped before the buffer handle is dropped
	unsafe fn push_td(&self) -> (TransferDescriptorId, int_buffers::FillingHandle) {
		let buf = self.buffers.get_buffer().expect("Unable to allocate interrupt buffer");
		let (first, last) = buf.get_phys_range();
		let td = self.controller.push_td( &self.id, hw::GeneralTdFlags::new_in().rounding().into(), first, last, ::kernel::futures::null_waker() );
		(td, buf)
	}

	fn poll_future(&self, cx: &mut ::core::task::Context) -> ::core::task::Poll< host::IntBuffer<'static> >
	{
		// 1. If the TD isn't scheduled, schedule now
		let mut lh = self.current_td.lock();
		if lh.is_none() {
			// SAFE: Saving the buffer handle in the endpoint structure
			*lh = Some(unsafe { self.push_td() });
		}
		// 2. Check state of current leader
		if let Some(remaining) = self.controller.td_complete(&lh.as_ref().unwrap().0)
		{
			// SAFE: Saving the buffer handle in the endpoint structure.
			let (td, buf_handle,) = ::core::mem::replace(&mut *lh, Some(unsafe { self.push_td() })).unwrap();
			let valid_len = self.buffers.max_packet_size() - remaining;
			log_debug!("Interrupt {:?}: {} bytes", td, valid_len);
			self.controller.release_td(td);
			// SAFE: Hardware is no longer accessing the buffer
			let filled_buffer = unsafe { buf_handle.filled(valid_len) };
			let rv = host::IntBuffer::new(filled_buffer)
				.ok().expect("OHCI interrupt buffer handle doesn't fit");
			::core::task::Poll::Ready(rv)
		}
		else
		{
			self.controller.td_update_waker(&lh.as_ref().unwrap().0, cx.waker());
			::core::task::Poll::Pending
		}
	}
}
impl ::core::ops::Drop for InterruptEndpointHandle
{
	fn drop(&mut self)
	{
		if let Some( (td_id, _buf_handle,) ) = self.current_td.get_mut().take()
		{
			todo!("Stop transfer {:?}", td_id);
		}
	}
}
impl host::InterruptEndpoint for InterruptEndpointHandle
{
	fn wait<'a>(&'a self) -> host::AsyncWaitIo<'a, host::IntBuffer<'a>>
	{
		struct Future<'a>(&'a InterruptEndpointHandle);
		impl<'a> ::core::future::Future for Future<'a>
		{
			type Output = host::IntBuffer<'a>;
			fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context) -> core::task::Poll<Self::Output> {
				self.0.poll_future(cx)
			}
		}
		host::AsyncWaitIo::<'a, _>::new(Future(self))
			.ok().expect("InterruptEndpointHandle::Future doesn't fit")
	}
}


struct BulkEndpointOut
{
	controller: ArefBorrow<HostInner>,
	id: EndpointId,
}
impl host::BulkEndpointOut for BulkEndpointOut
{
	fn send<'a>(&'a self, buffer: &'a [u8]) -> host::AsyncWaitIo<'a, usize>
	{
		struct Future<'a> {
			ep: &'a BulkEndpointOut,
			_bb_data: Option<::kernel::memory::virt::AllocHandle>,
			td_data: TransferDescriptorId,
			len: u16,
		}

		// SAFE: Bounce buffer is stored (TODO: Same as the control versions, this is slightly unsound with leaks)
		let (bounce_buf, td) = unsafe {
			let (bounce_buf, out_first_phys, out_last_phys) = self.controller.get_dma_todev(buffer);
			let td = self.controller.push_td( &self.id, hw::GeneralTdFlags::new_out().into(), out_first_phys, out_last_phys, ::kernel::futures::null_waker() );
			self.controller.kick_bulk();
			(bounce_buf, td)
			};

		return host::AsyncWaitIo::new(Future {
			ep: self,
			_bb_data: bounce_buf,
			td_data: td,
			len: buffer.len() as u16,
			}).unwrap_or_else(|e| host::AsyncWaitIo::new(Box::new(e)).ok().unwrap());
		impl ::core::future::Future for Future<'_>
		{
			type Output = usize;
			fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context) -> core::task::Poll<Self::Output> {
				if let Some(rem) = self.ep.controller.td_complete(&self.td_data)
				{
					log_trace!("Polling BulkEndpointOut::Future {:?}: Complete", self.td_data);
					::core::task::Poll::Ready(self.len as usize - rem)
				}
				else
				{
					log_trace!("Polling BulkEndpointOut::Future {:?}: Pending", self.td_data);
					self.ep.controller.td_update_waker(&self.td_data, cx.waker());
					::core::task::Poll::Pending
				}
			}
		}
		impl ::core::ops::Drop for Future<'_>
		{
			fn drop(&mut self)
			{
				log_trace!("Dropping BulkEndpointOut::Future {:?}", self.td_data);
				if self.ep.controller.td_complete(&self.td_data).is_none()
				{
					self.ep.controller.stop_td(&self.td_data);
				}
				self.ep.controller.release_td( ::core::mem::replace(&mut self.td_data, TransferDescriptorId::null()) );
			}
		}
	}
}
struct BulkEndpointIn
{
	controller: ArefBorrow<HostInner>,
	id: EndpointId,
}
impl host::BulkEndpointIn for BulkEndpointIn
{
	fn recv<'a>(&'a self, buffer: &'a mut [u8]) -> host::AsyncWaitIo<'a, usize>
	{
		struct Future<'a> {
			ep: &'a BulkEndpointIn,
			_bb_data: Option<::kernel::memory::virt::AllocHandle>,
			td_data: TransferDescriptorId,
			len: u16,
		}

		// SAFE: Bounce buffer is stored (TODO: Same as the control versions, this is slightly unsound with leaks)
		let (bounce_buf, td) = unsafe {
			let (bounce_buf, out_first_phys, out_last_phys) = self.controller.get_dma_fromdev(buffer);
			let td = self.controller.push_td( &self.id, hw::GeneralTdFlags::new_in().into(), out_first_phys, out_last_phys, ::kernel::futures::null_waker() );
			self.controller.kick_bulk();
			(bounce_buf, td)
			};

		return host::AsyncWaitIo::new(Future {
			ep: self,
			_bb_data: bounce_buf,
			td_data: td,
			len: buffer.len() as u16,
			}).unwrap_or_else(|e| host::AsyncWaitIo::new(Box::new(e)).ok().unwrap());

		impl ::core::future::Future for Future<'_>
		{
			type Output = usize;
			fn poll(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context) -> core::task::Poll<Self::Output> {
				if let Some(rem) = self.ep.controller.td_complete(&self.td_data)
				{
					::core::task::Poll::Ready(self.len as usize - rem)
				}
				else
				{
					self.ep.controller.td_update_waker(&self.td_data, cx.waker());
					::core::task::Poll::Pending
				}
			}
		}
		impl ::core::ops::Drop for Future<'_>
		{
			fn drop(&mut self)
			{
				//log_debug!("Dropping BulkEndpointIn::Future {:?}", self.td_data);
				if self.ep.controller.td_complete(&self.td_data).is_none()
				{
					self.ep.controller.stop_td(&self.td_data);
				}
				self.ep.controller.release_td( ::core::mem::replace(&mut self.td_data, TransferDescriptorId::null()) );
			}
		}
	}
}

impl ::kernel::device_manager::DriverInstance for BusDev
{
}

