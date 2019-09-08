// 
//! Open Host Controller Interface (OHCI) driver
#![no_std]
#![feature(linkage)]	// for module_define!
use kernel::prelude::*;
use kernel::_async3 as async;
use kernel::lib::mem::aref::{Aref,ArefBorrow};
use core::sync::atomic::{AtomicPtr,AtomicUsize,Ordering};

#[macro_use]
extern crate kernel;
extern crate usb_core;

mod hw;
mod pci;

module_define!{usb_ohci, [usb_core], init}

fn init()
{
	static PCI_DRIVER: pci::PciDriver = pci::PciDriver;
	::kernel::device_manager::register_driver(&PCI_DRIVER);
}

struct BusDev
{
	host: Aref<HostInner>,
}
struct UsbHost
{
	host: ArefBorrow<HostInner>,
}
struct HostInner
{
	io: IoWrapper,
	irq_handle: Option<::kernel::irqs::ObjectHandle>,
	hcca_handle: ::kernel::memory::virt::AllocHandle,
	nports: u8,
	waiter_idx: AtomicUsize,
	waiter_ptr: AtomicPtr<::kernel::sync::Queue<(usize,usize)>>,
}
struct IoWrapper(::kernel::device_manager::IOBinding);

/// Handle/index to an endpoint
struct EndpointId {
	// Group 0 is in the HCCA page (either in the interrupt graph or the buffers)
	group: u8,
	idx: u8
}
/// Index into a pool of transfer descriptors
struct TransferDescriptorId {
	// Group 0 is in the tail end of the HCCA
	group: u8,
	idx: u8,
}
struct EndpointMetadata {
	spinlock: ::kernel::sync::Spinlock<()>,
	tail: AtomicPtr<hw::GeneralTD>,
}


enum BounceBufferHandle<'a> {
	Direct(&'a [u8]),
	Bounced {
		orig_buf: Option<&'a mut [u8]>,
		bounce_buf: ::kernel::memory::virt::AllocHandle,
		},
}

impl BusDev
{
	fn new_boxed(irq: u32, io: ::kernel::device_manager::IOBinding) -> Result<Box<BusDev>, &'static str>
	{
		Ok(Box::new(BusDev {
			host: HostInner::new_aref(irq, io)?
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
		// SAFE: No side-effects
		log_notice!("Card {:?} version is {:#x}", io, unsafe { io.read_32(hw::Regs::HcRevision as usize * 4) });
		let io = IoWrapper(io);
		
		let fm_interval_val = io.read_reg(hw::Regs::HcFmInterval);
		let frame_interval = fm_interval_val & 0x3FFF;

		
		// Perform a hardware reset (and get controller from the firmware)
		// SAFE: Read is safe
		let hc_control = io.read_reg(hw::Regs::HcControl);
		if hc_control & 0x100 != 0
		{
			// SMM emulation
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
			io.write_reg(hw::Regs::HcCommandStatus, io.read_reg(hw::Regs::HcCommandStatus) | (1 << 0));
			// TODO: Wait for 10us
			//::kernel::time::wait_busy_microseconds(10);
			// - Restore the HcFmInterval value
			io.write_reg(hw::Regs::HcFmInterval, fm_interval_val);
			// - Set the bus back to UsbOperational
			io.write_reg(hw::Regs::HcControl, io.read_reg(hw::Regs::HcControl) & !0xC0 | 0x40);
		}


		// Allocate 'HCCA' (Host Controller Communication Area) and a bunch of other structures.
		// - Use the rest of that page as space for the interrupt structures and TDs
		// The following page contains:
		// -  256 byte HCCA
		// -  512 bytes for interupt graph
		// - 1280 bytes for 16+48 endpoints
		// - 2048 bytes for 64 transfer descriptors (with 16 bytes of metadata)
		let mut handle_hcca = ::kernel::memory::virt::alloc_dma(32, 1, "usb_ohci")?;
		let stop_endpoint_phys;
		// - Fill the interrupt lists
		{
			let r: &mut hw::IntLists = handle_hcca.as_mut(256);
			let mut next_level_phys = ::kernel::memory::virt::get_phys(r);
			let mut init_int_ep = |i, cnt, v: &mut hw::Endpoint| {
				use kernel::memory::PAddr;
				if i == 0 {
					next_level_phys += (cnt * ::core::mem::size_of::<hw::Endpoint>()) as PAddr;
				}
				v.next_ed = (next_level_phys + (i as PAddr / 2) * ::core::mem::size_of::<hw::Endpoint>() as PAddr) as u32;
				v.flags = 1 << 14;
				};
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
				*d = int_base + idx * ::core::mem::size_of::<hw::Endpoint>() as u32;
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
			// - 7/6: HostControllerFunctionalState (=01 UsbOperational)
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
			waiter_idx: Default::default(),
			waiter_ptr: Default::default(),
			});
		
		// Bind interrupt
		{
			struct RawSend<T: Send>(*const T);
			unsafe impl<T: Send> Send for RawSend<T> {}
			let ret_raw = RawSend(&*inner_aref);
			// SAFE: Pointer _should_ be valid as long as this IRQ binding exists
			Aref::get_mut(&mut inner_aref).unwrap().irq_handle = Some(::kernel::irqs::bind_object(irq, Box::new(move || unsafe { (*ret_raw.0).handle_irq() } )));
		}

		::usb_core::register_host(Box::new(UsbHost { host: inner_aref.borrow() }));
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
				log_debug!("WritebackDoneHead - ");
				// TODO: Clear the contents of HccaDoneHead, releasing and completing those TDs
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
						let host_idx = self.waiter_idx.load(Ordering::Relaxed);
						let queue_ptr = self.waiter_ptr.load(Ordering::Relaxed);
						// SAFE: If this is set, it's to a &'static
						unsafe { (*queue_ptr).push( (host_idx, i as usize) ); }
					}
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

	fn get_ed_pointer(&self, id: &EndpointId) -> *const hw::Endpoint {
		if id.group == 0 {
			let ofs = (id.idx as usize) * ::core::mem::size_of::<hw::Endpoint>();
			assert!(ofs >= 256);
			assert!(ofs < 2048);
			self.hcca_handle.as_ref(ofs)
		}
		else {
			todo!("get_ed_pointer: Alternate pools for endpoint descriptors");
		}
	}
	fn get_general_td_pointer(&self, id: &TransferDescriptorId) -> *const hw::GeneralTD {
		if id.group == 0 {
			let ofs = (id.idx as usize) * ::core::mem::size_of::<hw::GeneralTD>();
			assert!(ofs >= 2048);
			assert!(ofs < 4096);
			self.hcca_handle.as_ref(ofs)
		}
		else {
			todo!("get_general_td_pointer: Alternate pools for transfer descriptors");
		}
	}
	fn get_endpoint_meta(&self, id: &EndpointId) -> &EndpointMetadata {
		todo!("get_endpoint_meta");
	}

	/// Register an interrupt endpoint
	fn register_interrupt_ed(&self, period_pow_2: usize, flags: u32) -> EndpointId
	{
		// 1. Find a low-load slot of this period
		// 2. Check if the placeholder is in use
		// 3. If not, allocate a new endpoint descriptor and put it after the placeholder
		todo!("register_interrupt_ed");
	}
	/// Register a general-purpose endpoint descriptor and add it to the control queue
	fn register_control_ed(&self, flags: u32) -> EndpointId
	{
		todo!("register_control_ed");
	}
	/// Register a general-purpose endpoint descriptor and add it to the bulk queue
	fn register_bulk_ed(&self, flags: u32) -> EndpointId
	{
		todo!("register_bulk_ed");
	}
	/// Allocate a new TD
	fn allocate_td(&self, flags: u32) -> TransferDescriptorId
	{
		use core::sync::atomic::AtomicU32;
		// Iterate over all avaliable pools
		const SIZE: usize = ::core::mem::size_of::<hw::GeneralTD>();
		for i in (2048 / SIZE .. 4096 / SIZE)
		{
			let ofs = i * SIZE;
			log_debug!("allocate_td: i={} ofs={}", i, ofs);
			// Do a compare+set of the flags field with the new value (with some masking)
			// SAFE: I assume so? (TODO: Check)
			let flags_ptr: &AtomicU32 = unsafe { &*(self.hcca_handle.as_ref(ofs) as *const u32 as *const AtomicU32) };
			if flags_ptr.compare_and_swap(0, flags | 1, Ordering::SeqCst) == 0
			{
				return TransferDescriptorId { group: 0, idx: i as u8 };
			}
		}
		todo!("allocate_td - alternate pools, main is exhausted");
	}
	unsafe fn push_td(&self, ep: &EndpointId, flags: u32, first_byte: u32, last_byte: u32, async_handle: async::ObjectHandle) -> TransferDescriptorId
	{
		// 1. Allocate a new transfer descriptor
		let td_handle = self.allocate_td(flags);
		let td_ptr: *mut hw::GeneralTD = self.get_general_td_pointer(&td_handle) as *mut _;
		// - Fill it
		//td_ptr.fill(first_byte, last_byte, ::core::mem::transmute::<_, usize>(async_handle) as u64);
		{
			::core::ptr::write(&mut (*td_ptr).cbp, first_byte);
			::core::ptr::write(&mut (*td_ptr).buffer_end, last_byte);
			// - Store the (single pointer) async handle in the 64-bit meta field
			::core::ptr::write(&mut (*td_ptr).meta_async_handle, ::core::mem::transmute::<_, usize>(async_handle) as u64);
		}
		// 2. Add to the end of the endpoint's queue.
		let ed = self.get_ed_pointer(ep) as *mut hw::Endpoint;
		// - Set SKIP in flags
		// - Get the metadata for this endpoint and lock it (spinlock)
		let epm = self.get_endpoint_meta(ep);
		let _lh = epm.spinlock.lock();
		// - If the NextP is 0, set NextP = phys(td)
		// > TODO: Atomic compare_and_swap
		if (*ed).head_ptr == 0 {
			(*ed).head_ptr = ::kernel::memory::virt::get_phys(td_ptr) as u32;
			epm.tail.store(td_ptr, Ordering::SeqCst);
		}
		// - Else, set EP_Meta.tail.NextP = phys(td)
		else {
			// TODO: Possible race, if hardware is processing the TDs and is about to write back NextP between the above read and the write here.
			let prev_tail = epm.tail.swap(td_ptr, Ordering::SeqCst);
			(*prev_tail).next_td = ::kernel::memory::virt::get_phys(td_ptr) as u32;
		}

		td_handle
	}

	// Get a handle for a DMA output
	fn get_dma_todev<'long,'short>(&self, buffer: async::WriteBufferHandle<'long,'short>) -> (BounceBufferHandle<'long>, u32, u32)
	{
		match buffer
		{
		async::WriteBufferHandle::Long(p) => {
			let start_phys = ::kernel::memory::virt::get_phys(p.as_ptr());
			let last_phys = ::kernel::memory::virt::get_phys(&p[p.len()-1]);
			if start_phys > 0xFFFF_FFFF || last_phys > 0xFFFF_FFFF {
				// An address is more than 32-bits, bounce
			}
			else if start_phys & !0xFFF != last_phys & !0xFFF && (0x1000 - (start_phys & 0xFFF) + last_phys & 0xFFF) as usize != p.len() {
				// The buffer spans more than two pages, bounce
			}
			else {
				// Good
				return ( BounceBufferHandle::Direct(p), start_phys as u32, last_phys as u32);
			}
			todo!("Bounce buffer - long lifetime");
			},
		async::WriteBufferHandle::Short(_p) => {
			todo!("Bounce buffer - short lifetime");
			},
		}
	}
	// Get a handle for a DMA input
	fn get_dma_fromdev<'a>(&self, p: &'a mut [u8]) -> (BounceBufferHandle<'a>, u32, u32)
	{
		{
			let start_phys = ::kernel::memory::virt::get_phys(p.as_ptr());
			let last_phys = ::kernel::memory::virt::get_phys(&p[p.len()-1]);
			if start_phys > 0xFFFF_FFFF || last_phys > 0xFFFF_FFFF {
				// An address is more than 32-bits, bounce
			}
			else if start_phys & !0xFFF != last_phys & !0xFFF && (0x1000 - (start_phys & 0xFFF) + last_phys & 0xFFF) as usize != p.len() {
				// The buffer spans more than two pages, bounce
			}
			else {
				// Good
				return ( BounceBufferHandle::Direct(p), start_phys as u32, last_phys as u32);
			}
			todo!("Bounce buffer for read");
		}
	}

	fn get_port_reg(&self, port: usize) -> hw::Regs
	{
		assert!(port < 16);
		assert!(port < self.nports as usize);
		// SAFE: Bounds are checked to fit within the alowable range for the enum
		unsafe { ::core::mem::transmute(hw::Regs::HcRhPortStatus0 as usize + port) }
	}
}

use ::usb_core::host::{EndpointAddr, PortFeature, Handle};
use ::usb_core::host::{InterruptEndpoint, IsochEndpoint, ControlEndpoint, BulkEndpoint};
impl ::usb_core::host::HostController for UsbHost
{
	/// Begin polling an endpoint at the given rate (buffer used is allocated by the driver to be the interrupt endpoint's size)
	fn init_interrupt(&self, endpoint: EndpointAddr, period_ms: usize, _waiter: async::ObjectHandle) -> Handle<dyn InterruptEndpoint> {
		todo!("init_interrupt({:?}, period_ms={}", endpoint, period_ms);
	}
	fn init_isoch(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn IsochEndpoint> {
		todo!("init_isoch({:?}, max_packet_size={})", endpoint, max_packet_size);
	}
	fn init_control(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn ControlEndpoint> {
		// Allocate an endpoint
		let ptr = self.host.register_control_ed(
			  (endpoint.dev_addr() & 0x7F) as u32
			| ((endpoint.endpt() & 0xF) << 7) as u32
			| (0b00 << 11)	// Direction - Use TD
			| (0b0 << 13)	// Speed (TODO)
			| (0b0 << 14)	// Skip - clear
			| (0b0 << 15)	// Format - 0=control/bulk/int
			| ((max_packet_size & 0xFFFF) << 16) as u32
			);

		Handle::new(ControlEndpointHandle {
			controller: self.host.reborrow(),
			id: ptr,
			}).ok().unwrap()
	}
	fn init_bulk(&self, endpoint: EndpointAddr, max_packet_size: usize) -> Handle<dyn BulkEndpoint> {
		todo!("init_bulk({:?}, max_packet_size={})", endpoint, max_packet_size);
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
	fn set_root_waiter(&mut self, waiter: &'static ::kernel::sync::Queue<(usize,usize)>, my_idx: usize) {
		// 1. Store the waiter pointer
		self.host.waiter_idx.store(my_idx, Ordering::SeqCst);
		self.host.waiter_ptr.store(waiter as *const _ as *mut _, Ordering::SeqCst);
		// 2. For each connected port, push an event/item
		// - Don't worry about duplicates from interrupts...
		for i in 0 .. self.host.nports as usize
		{
			let v = self.host.io.read_reg(self.host.get_port_reg(i));
			log_debug!("set_root_waiter: Port {} - v={:#x}", i, v);
			if v & 0x1 != 0 {
				waiter.push( (my_idx, i) );
			}
		}
	}
}
struct ControlEndpointHandle {
	controller: ArefBorrow<HostInner>,
	id: EndpointId,
}
impl ControlEndpoint for ControlEndpointHandle
{
	fn out_only<'a, 's>(&'s self, async: async::ObjectHandle, mut stack: async::StackPush<'a, 's>, setup_data: async::WriteBufferHandle<'s, '_>, out_data: async::WriteBufferHandle<'s, '_>)
	{
		// Get (potentially bounced) data handles
		let (setup_buf, setup_first_phys, setup_last_phys) = self.controller.get_dma_todev(setup_data);
		let (out_buf, out_first_phys, out_last_phys) = self.controller.get_dma_todev(out_data);

		// SAFE: The buffers stay valid because the handles are moved into the async closure.
		unsafe {
			self.controller.push_td( &self.id, (0b00 << 19) /* setup */ | (7 << 21) /* no int */, setup_first_phys, setup_last_phys, async.clone() );
			self.controller.push_td( &self.id, (0b01 << 19) /* out */ | (0 << 21) /* immediate int */, out_first_phys, out_last_phys, async.clone() );
		}
		stack.push_closure(move |_async, _stack, out_bytes| {
			// - Capture buffer handles so they stay valid
			let _ = setup_buf;
			let _ = out_buf;
			// - Pass the result down the chain.
			Some(out_bytes)
			}).expect("Stack exhaustion in ohci::ControlEndpointHandle::out_only");
	}
	fn in_only<'a, 's>(&'s self, async: async::ObjectHandle, mut stack: async::StackPush<'a, 's>, setup_data: async::WriteBufferHandle<'s, '_>, in_buf: &'s mut [u8])
	{
		// Get (potentially bounced) data handles
		let (setup_buf, setup_first_phys, setup_last_phys) = self.controller.get_dma_todev(setup_data);
		let (data_buf, data_first_phys, data_last_phys) = self.controller.get_dma_fromdev(in_buf);
		// SAFE: The buffers stay valid because they're moved into the async closure.
		unsafe {
			self.controller.push_td( &self.id, (0b00 << 19) /* setup */ | (7 << 21) /* no int */, setup_first_phys, setup_last_phys, async.clone() );
			self.controller.push_td( &self.id, (0b10 << 19) /* in */ | (0 << 21) /* immediate int */, data_first_phys, data_last_phys, async.clone() );
		}
		stack.push_closure(move |_async, _stack, out_bytes| {
			// - Capture buffer handles so they stay valid
			let _ = setup_buf;
			let _ = data_buf;
			if let BounceBufferHandle::Bounced { ref orig_buf, ref bounce_buf } = data_buf {
				todo!("Copy data back out of the bounce buffer - {:?} -> {:p}", bounce_buf, orig_buf);
			}
			// - Pass the result down the chain.
			Some(out_bytes)
			}).expect("Stack exhaustion in ohci::ControlEndpointHandle::in_only");
	}
}

impl ::kernel::device_manager::DriverInstance for BusDev
{
}

