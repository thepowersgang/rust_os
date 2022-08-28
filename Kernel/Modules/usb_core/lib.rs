// "Tifflin" Kernel - USB interface core
// - By John Hodge (Mutabah / thePowersGang)
//
// Modules/usb_core/lib.rs
//! USB Core library (provides interfaces between device and host drivers)
#![no_std]
#![feature(linkage)]	// for module_define!
#![feature(try_blocks)]
#![feature(arbitrary_self_types)]
use kernel::prelude::*;
use kernel::lib::mem::aref::{Aref,ArefBorrow};
use kernel::sync::Mutex;

#[macro_use]
extern crate kernel;
extern crate stack_dst;
extern crate utf16;

module_define!{usb_core, [], init}

fn init()
{
}

mod hub;
pub mod host;
pub mod device;
pub mod handle;
pub mod hw_decls;

/// Reference to a hub
#[derive(Clone)]
enum HubRef
{
	//Root(ArefBorrow<Host>),
	Root(HostRef),
	Device(ArefBorrow<hub::HubDevice<'static>>),
}

/// A reference to a host
#[derive(Clone)]
struct HostRef(*const Host);
unsafe impl Send for HostRef where Host: Sync {
}
unsafe impl Sync for HostRef where Host: Sync {
}
impl HostRef
{
	// UNSAFE: Caller must ensure that pointed-to host outlives the HostRef
	pub unsafe fn new(p: *const Host) -> Self {
		HostRef(p)
	}
}
impl core::ops::Deref for HostRef {
	type Target = Host;
	fn deref(&self) -> &Host {
		// SAFE: Contract in `HostRef::new`
		unsafe { &*self.0 }
	}
}

#[derive(Default)]
struct AddressPool
{
	next_id: u8,
	used_ids: [u8; 128/8],
}
/// Representation of a host/bus
/// - Used to hold the device address allocation logic/structures
struct Host
{
	driver: Box<dyn host::HostController>,
	addresses: Mutex<AddressPool>,

	// TODO: EHCI needs a different endpoint handle for 1.0 devices (different speeds)
	endpoint_zero_handle: ControlEndpoint,
	endpoint_zero_lock: ::kernel::futures::Mutex<()>,
	
	//root_ports: OnceCell<Vec<Port>>,
	root_ports: Vec<PortState>,
	
	//device_workers: [Mutex<Option<core::pin::Pin<Box<dyn core::future::Future<Output=()> + Send>>>>; 255],
	device_workers: Vec< Mutex<Option<core::pin::Pin<Box<dyn core::future::Future<Output=()> + Send>>>> >,

	// Hub port speed information
	// - This is required for EHCI
}
struct HostEnt
{
	_host: Aref<Host>,
	_worker: kernel::threads::WorkerThread,
}

static HOST_LIST: Mutex<Vec<HostEnt>> = ::kernel::sync::Mutex::new(Vec::new());

/// Add a new host controller/bus to the system
pub fn register_host(driver: Box<dyn host::HostController>, nports: u8)
{
	let host = Aref::new(Host {
		addresses: ::kernel::sync::Mutex::new(AddressPool {
			next_id: 1,
			used_ids: [0; 128/8],
			}),
		endpoint_zero_handle: ControlEndpoint {
			inner: driver.init_control(crate::host::EndpointAddr::new(0, 0), 64),
			},
		endpoint_zero_lock: Default::default(),
		root_ports: {
			let mut v = Vec::new();
			v.resize_with(nports as usize, || PortState::new());
			v
			},
		//device_workers: Default::default(),
		device_workers: {
			let mut v = Vec::new();
			v.resize_with(255, Default::default);
			v
			},

		driver: driver,
		});

	let hb = host.borrow();
	let mut lh = HOST_LIST.lock();
	lh.push(HostEnt {
		_host: host,
		_worker: ::kernel::threads::WorkerThread::new("USB Host", move || host_worker(hb)),
		});
}

fn host_worker(host: ArefBorrow<Host>)
{
	let mut host_async = host.root_event_task();
	// SAFE: Not moved until it's dropped
	let mut host_async = unsafe { core::pin::Pin::new_unchecked(&mut host_async) };
	kernel::futures::runner(|context| {
		use ::core::future::Future;
		use ::core::task::Poll;
		match host_async.as_mut().poll(context)
		{
		Poll::Ready( () ) => panic!("Host root task completed"),
		Poll::Pending => {},
		}
		// The following list has a fixed number of entries (255), each with its own lock
		// Added devices will just claim an unused entry, which won't be locked (as the worker only has the hub locked)
		for p in host.device_workers.iter()
		{
			let mut p = p.lock();
			let done = if let Some(ref mut p) = *p {
					matches!(p.as_mut().poll(context), Poll::Ready(()))
				}
				else {
					false
				};
			if done {
				log_debug!("Device worker complete");
				*p = None;
			}
		}
		None::<()>
	});
}

impl HubRef
{
	fn host(&self) -> &HostRef {
		match self
		{
		&HubRef::Root  (ref h) => h,
		&HubRef::Device(ref h) => &h.host,
		}
	}

	fn power_stable_time_ms(&self) -> u32
	{
		match self
		{
		&HubRef::Root  (ref _h) => todo!("power_stable_time_ms for root"),
		&HubRef::Device(ref h) => h.power_stable_time_ms(),
		}

	}
	async fn set_port_feature(&self, port_idx: usize, feat: host::PortFeature)
	{
		match self
		{
		&HubRef::Root  (ref h) => h.driver.set_port_feature(port_idx, feat),
		&HubRef::Device(ref h) => h.set_port_feature(port_idx, feat).await,
		}
	}
	async fn clear_port_feature(&self, port_idx: usize, feat: host::PortFeature)
	{
		match self
		{
		&HubRef::Root  (ref h) => h.driver.clear_port_feature(port_idx, feat),
		&HubRef::Device(ref h) => h.clear_port_feature(port_idx, feat).await,
		}
	}
	async fn get_port_feature(&self, port_idx: usize, feat: host::PortFeature) -> bool
	{
		match self
		{
		&HubRef::Root  (ref h) => h.driver.get_port_feature(port_idx, feat),
		&HubRef::Device(ref h) => h.get_port_feature(port_idx, feat).await,
		}
	}
}

struct PortState
{
	is_connected: ::core::sync::atomic::AtomicBool,
}
impl PortState
{
	fn new() -> Self {
		PortState {
			is_connected: Default::default(),
		}
	}

	fn signal_connected(&self, hub: HubRef, port_idx: u8)
	{
		if self.is_connected.swap(true, ::core::sync::atomic::Ordering::Relaxed) {
			log_notice!("signal_connected: {} connected while already connected?", port_idx);
		}
		else {
			// TODO: Record the address, so it can be removed/signaled?
			hub.clone().host().add_device(move |addr| PortDev::new(hub, port_idx, addr).worker());
		}
	}
}
struct PortDev
{
	hub: HubRef,
	port_idx: u8,
	addr: u8,
}
impl PortDev
{
	pub fn new(hub: HubRef, port_idx: u8, addr: u8) -> PortDev {
		PortDev {
			hub,
			port_idx,
			addr,
			}
	}
	fn host(&self) -> &HostRef {
		self.hub.host()
	}

	fn set_port_feature(&self, feat: host::PortFeature) -> impl core::future::Future<Output=()> + '_ {
		self.hub.set_port_feature(self.port_idx as usize, feat)
	}
	#[allow(dead_code)]
	fn clear_port_feature(&self, feat: host::PortFeature) -> impl core::future::Future<Output=()> + '_ {
		self.hub.clear_port_feature(self.port_idx as usize, feat)
	}
	fn get_port_feature(&self, feat: host::PortFeature) -> impl core::future::Future<Output=bool> + '_ {
		self.hub.get_port_feature(self.port_idx as usize, feat)
	}
	//async fn getclear_port_feature(&self, feat: host::PortFeature) -> bool {
	//	let rv = self.get_port_feature(feat).await;
	//	if rv {
	//		self.clear_port_feature(feat).await;
	//	}
	//	rv
	//}

	async fn initialise_port(&self, address: u8) -> Result<(),()>
	{
		log_debug!("initialise_port({address})");
		let addr0_handle = self.host().get_address_zero().await;
		if ! self.get_port_feature(host::PortFeature::Power).await
		{
			log_debug!("initialise_port({address}): Turning on power");
			// Set power
			self.set_port_feature(host::PortFeature::Power).await;
			// Wait for the hub-provided stable time
			kernel::futures::msleep( self.hub.power_stable_time_ms() as usize ).await;
		}

		// TODO: Resetting the port can cause the connection status to change (leading to an infinite loop)

		// Request a port reset
		self.set_port_feature(host::PortFeature::Reset).await;
		kernel::futures::msleep(50).await;
		// Clear `Reset` if it's not cleared on its own.
		if self.get_port_feature(host::PortFeature::Reset).await {
			self.clear_port_feature(host::PortFeature::Reset).await;
			// Wait for the reset clear
			let timeout = kernel::time::ticks() + 50;
			while self.get_port_feature(host::PortFeature::Reset).await {
				if kernel::time::ticks() > timeout {
					log_error!("initialise_port({address}): Timeout waiting for reset");
					return Err( () );
				}
				kernel::futures::msleep(2).await;
			}
		}
		// TODO: EHCI may want to defer this to another controller

		// TODO: Why is there another sleep here?
		kernel::futures::msleep(2).await;

		// Enable the port if the hub hasn't done that for us
		if ! self.get_port_feature(host::PortFeature::Enable).await {
			log_debug!("initialise_port({address}): Enabling");
			self.set_port_feature(host::PortFeature::Enable).await;
			
			// Wait for the enable to set
			let timeout = kernel::time::ticks() + 50;
			while !self.get_port_feature(host::PortFeature::Enable).await {
				if kernel::time::ticks() > timeout {
					log_error!("initialise_port({address}): Timeout waiting for enable");
					return Err( () );
				}
				kernel::futures::msleep(5).await;
			}
		}
		addr0_handle.send_setup_address(address).await;
		log_debug!("initialise_port({address}): Done");
		Ok( () )
	}

	async fn enumerate<'a>(&self, ep0: &'a ControlEndpoint) -> Result<Vec<Interface<'a>>, &'static str>
	{
		let dev_descr: hw_decls::Descriptor_Device = ep0.read_descriptor(/*index*/0).await?;
		log_debug!("dev_descr = {:?}", dev_descr);
		log_debug!("dev_descr.usb_version = {:x}", dev_descr.usb_version);
		log_debug!("dev_descr.vendor_id/device_id = {:04x}:{:04}", dev_descr.vendor_id, dev_descr.device_id);
		let mfg_str = ep0.read_string(dev_descr.manufacturer_str).await?;
		let prod_str = ep0.read_string(dev_descr.product_str).await?;
		let ser_str = ep0.read_string(dev_descr.serial_number_str).await?;
		log_debug!("dev_descr.manufacturer_str = #{} {}", dev_descr.manufacturer_str, mfg_str);
		log_debug!("dev_descr.product_str = #{} {}", dev_descr.product_str, prod_str);
		log_debug!("dev_descr.serial_number_str = #{} {}", dev_descr.serial_number_str, ser_str);

		log_notice!("DEVICE {:04x}:{:04x} \"{}\" \"{}\" SN \"{}\"",
			dev_descr.vendor_id, dev_descr.device_id,
			mfg_str, prod_str, ser_str,
			);

		// Enumerate all configurations
		for idx in 0 .. dev_descr.num_configurations
		{
			let base_cfg: hw_decls::Descriptor_Configuration = ep0.read_descriptor(idx).await?;
			let cfg_str = ep0.read_string(base_cfg.configuration_str).await?;
			log_debug!("cfg[{}] = {:?} ({:?})", idx, cfg_str, base_cfg);
		}

		if dev_descr.num_configurations > 1 {
			// TODO: Pick an alternative configuration (if there's more than 1)
			// - Pick the first one that finds a driver?
		}

		// Just hard-code configuration 0 for now
		self.set_configuration(ep0, 0).await
	}

	async fn set_configuration<'a>(&self, ep0: &'a ControlEndpoint, idx: u8) -> Result<Vec<Interface<'a>>, &'static str>
	{
		// Get the base configuration descriptor
		let base_cfg: hw_decls::Descriptor_Configuration = ep0.read_descriptor(idx).await?;
		// - Fetch the full descriptor (includes interfaces and endpoints)
		let mut cfg_buf = vec![0; base_cfg.total_length as usize];
		ep0.read_descriptor_raw(<hw_decls::Descriptor_Configuration as hw_decls::Descriptor>::TYPE, idx, &mut cfg_buf).await?;
		let other_descriptors = &cfg_buf[base_cfg.length as usize..];

		// Count the number of interfaces and pre-allocate the return list
		let n_ints = hw_decls::IterDescriptors(other_descriptors)
			.map(hw_decls::DescriptorAny::from_bytes)
			.filter(|v| is!(v, Ok(hw_decls::DescriptorAny::Interface(..))))
			.count();
		let mut interfaces = Vec::with_capacity(n_ints);

		// Iterate descriptors, looking for interfaces
		// - Tracks the previous interface and the start of the intervening descriptor list
		let mut it = hw_decls::IterDescriptors(other_descriptors);
		let mut last_int: Option<(hw_decls::Descriptor_Interface, &[u8],)> = None;
		while let Some(desc) = it.next()
		{
			let desc = hw_decls::DescriptorAny::from_bytes(desc);
			if let Ok(hw_decls::DescriptorAny::Interface(v)) = desc
			{
				let s = ep0.read_string(v.interface_str).await?;
				log_debug!("Interface string '{}'", s);
				if let Some( (v,start) ) = last_int.take()
				{
					// Note: minus 9 so it excludes the current iteration's interface
					let endpoint_list = &start[..start.len() - it.0.len() - 9];
					interfaces.push( self.spawn_interface(&ep0, &v, endpoint_list) );
				}
				last_int = Some( (v, it.0) );
			}
		}
		if let Some( (v,start) ) = last_int.take()
		{
			let endpoint_list = &start[..start.len() - it.0.len()];
			interfaces.push( self.spawn_interface(&ep0, &v, endpoint_list) );
		}
		Ok(interfaces)
	}

	fn spawn_interface<'a>(&self, endpoint_0: &'a ControlEndpoint, int_desc: &hw_decls::Descriptor_Interface, descriptors: &[u8]) -> Interface<'a>
	{
		let full_class
			= (int_desc.interface_class as u32) << 16
			| (int_desc.interface_sub_class as u32) << 8
			| (int_desc.interface_protocol as u32) << 0
			;
		// - Look up using the interface class specs
		//  > May also want specialised drivers?
		// - If a driver can't be found, what do?

		// Idea:
		// - Each interface is constructed as-is according to the descriptors
		// - Store the interfaces in `self` (or return from `enumerate`)
		// - Assign a driver to the constructed interface
		let mut endpts = Vec::with_capacity(int_desc.num_endpoints as usize);
		for desc in hw_decls::IterDescriptors(descriptors).map(hw_decls::DescriptorAny::from_bytes)
		{
			if let Ok(hw_decls::DescriptorAny::Endpoint(ep_desc)) = desc
			{
				let ep_num = ep_desc.address & 0xF;
				let ep_dir_in = ep_desc.address & 0x80 != 0;
				let ep_type = (ep_desc.attributes & 0x3) >> 0;
				let max_packet_size = (ep_desc.max_packet_size.0 as u16) | (ep_desc.max_packet_size.1 as u16 & 0x03) << 8;
				let poll_period = ep_desc.max_polling_interval;
				log_debug!("EP {} {} {} MPS={}",
					ep_num,
					["OUT","IN"][ep_dir_in as usize],
					["Control","Isoch","Bulk","Interrupt"][ep_type as usize],
					max_packet_size,
					);
				endpts.push(match ep_type
					{
					0 => Endpoint::Control(ControlEndpoint::new(self.host(), self.addr, ep_num, max_packet_size as usize)),
					1 => todo!("Isoch endpoint"),//Endpoint::Isoch(IsochEndpoint::new(self.host(), self.addr, ep_num, max_packet_size, ep_dir_in, ...)),
					2 => if ep_dir_in {
							Endpoint::BulkIn(BulkEndpointIn::new(self.host(), self.addr, ep_num, max_packet_size as usize))
						}
						else {
							Endpoint::BulkOut(BulkEndpointOut::new(self.host(), self.addr, ep_num, max_packet_size as usize))
						},
					3 => if ep_dir_in {
							Endpoint::Interrupt(InterruptEndpoint::new(self.host(), self.addr, ep_num, max_packet_size as usize, poll_period as usize))
						}
						else {
							todo!("Out interrupt endpoint?");
						},
					_ => unreachable!("endpoint type"),
					});
			}
		}

		// NOTE: Hubs need the host reference, so have explicit code
		if full_class & 0xFF0000 == 0x090000 {
			return Interface::Bound(hub::start_device(self.host().clone(), endpoint_0, endpts).into())
		}
		// Locate a suitable driver
		match crate::device::find_driver(0,0, full_class)
		{
		Some(d) => {
			// Start the device
			Interface::Bound(d.start_device(endpoint_0, endpts, descriptors).into())
			},
		None => {
			log_notice!("No driver for class={:06x}", full_class);
			// If a driver can't be found, save the endpoints for later (and the descriptor data)
			Interface::Unknown(endpts, descriptors.to_owned())
			},
		}
	}

	async fn worker(self)
	{
		match self.initialise_port(self.addr).await {
		Ok(()) => {},
		Err(()) => return,
		}
		
		let ep0 = ControlEndpoint::new(self.host(), self.addr, /*ep_num=*/0, /*max_packet_size=*/64);
		// Enumerate device
		let interfaces = match self.enumerate(&ep0).await
			{
			Ok(v) => v,
			Err(e) => panic!("PortDev::worker({}) Device enumeration error: {}", self.addr, e),
			};

		log_debug!("{} interfaces", interfaces.len());
		// Await on a wrapper of the interfaces
		struct FutureVec<'a>(Vec<Interface<'a>>);
		impl<'a> ::core::future::Future for FutureVec<'a>
		{
			type Output = ();
			fn poll(self: ::core::pin::Pin<&mut Self>, cx: &mut ::core::task::Context<'_>) -> ::core::task::Poll<()> {
				// SAFE: The `Vec` is never added to after this is called, so pinning is maintained
				for (i,v) in Iterator::enumerate(unsafe { self.get_unchecked_mut().0.iter_mut() }) {
					match v
					{
					Interface::Unknown(..) => {
						log_debug!("interface {} unknown", i);
						},
					Interface::Bound(ref mut inst) =>
						match inst.as_mut().poll(cx)
						{
						::core::task::Poll::Pending => {},
						::core::task::Poll::Ready( () ) => todo!("Handle device future completing"),
						},
					}
				}
				::core::task::Poll::Pending
			}
		}
		FutureVec(interfaces).await;
	}
}

/// Representation of an active device interface
enum Interface<'a>
{
	/// No fitting driver (yet) - save the endpoints and descriptor data
	Unknown(Vec<Endpoint>, Vec<u8>),
	/// Started driver
	Bound(::core::pin::Pin<crate::device::Instance<'a>>),
}

pub enum Endpoint
{
	Control(ControlEndpoint),
	Interrupt(InterruptEndpoint),
	BulkIn(BulkEndpointIn),
	BulkOut(BulkEndpointOut),
}

pub struct InterruptEndpoint
{
	inner: crate::host::Handle<dyn crate::host::InterruptEndpoint>,
}
impl InterruptEndpoint
{
	fn new(host: &Host, addr: u8, ep_num: u8, max_packet_size: usize, polling_interval: usize) -> Self {
		Self {
			inner: host.driver.init_interrupt(crate::host::EndpointAddr::new(addr, ep_num), max_packet_size, polling_interval),
			}
	}

	pub async fn wait<'a>(&'a self) -> InterruptBuffer<'a> {
		InterruptBuffer {
			inner: self.inner.wait().await,
			}
	}
}
pub struct InterruptBuffer<'a>
{
	inner: crate::host::IntBuffer<'a>,
}
impl<'a> ::core::ops::Deref for InterruptBuffer<'a> {
	type Target = [u8];
	fn deref(&self) -> &[u8] {
		self.inner.get()
	}
}

pub struct ControlEndpoint
{
	inner: crate::host::Handle<dyn crate::host::ControlEndpoint>,
}
impl ControlEndpoint
{
	fn new(host: &Host, addr: u8, ep_num: u8, max_packet_size: usize) -> ControlEndpoint {
		ControlEndpoint {
			inner: host.driver.init_control(crate::host::EndpointAddr::new(addr, ep_num), max_packet_size),
			}
	}
	pub async fn read_request(&self, request_type: u8, request_num: u8, value: u16, index: u16, buf: &mut [u8])
	{
		let hdr = hw_decls::DeviceRequest {
			req_type: request_type,
			req_num: request_num,
			value: value,
			index: index,
			length: buf.len() as u16,
			};
		let hdr = hdr.to_bytes();
		let read_len = self.inner.in_only(&hdr, buf).await;
		assert_eq!(read_len, buf.len());
	}
	pub async fn read_descriptor_raw(&self, ty: u16, index: u8, buf: &mut [u8]) -> Result<usize,&'static str>
	{
		//log_trace!("read_descriptor_raw: (ty={:#x}, index={}, buf={}b)", ty, index, buf.len());
		let exp_length = buf.len();
		let hdr = hw_decls::DeviceRequest {
			// TODO: These high bits of `ty` aren't present in the returned structure - what are they again?
			req_type: 0x80 | ((ty >> 8) as u8 & 0x3) << 5 | (ty >> 12) as u8 & 3,
			req_num: 6,	// GET_DESCRIPTOR
			value: (ty << 8) | index as u16,
			index: 0,	// TODO: language ID
			length: exp_length as u16,
			};
		let hdr = hdr.to_bytes();
		let res_len = self.inner.in_only(&hdr, buf).await;

		Ok(res_len)
	}
	pub async fn read_descriptor<T>(&self, index: u8) -> Result<T,&'static str>
	where
		T: hw_decls::Descriptor
	{
		let exp_length = ::core::mem::size_of::<T>();
		//log_trace!("read_descriptor: (index={}): exp_length={}", index, exp_length);
		let mut out_data = [0u8; 256];
		let res_len = self.read_descriptor_raw(T::TYPE, index, &mut out_data[..exp_length]).await?;

		match T::from_bytes(&out_data[..res_len])
		{
		Ok(v) => Ok(v),
		Err(hw_decls::ParseError) => {
			log_error!("read_descriptor<{}>: Parse error on descriptor #{}: {:?}", ::core::any::type_name::<T>(), index, ::kernel::logging::HexDump(&out_data[..res_len]));
			Err("hw_decls::ParseError")
			},
		}
	}

	// TODO: Better return type?
	pub async fn read_string(&self, index: u8) -> Result<String,&'static str>
	{
		if index == 0 {
			return Ok(String::new());
		}
		let desc: hw_decls::Descriptor_String = self.read_descriptor(index).await?;
		match ::utf16::Str16::new(&desc.utf16[..desc.length as usize / 2 - 1])
		{
		Some(v) => Ok( format!("{}", v) ),
		None => Err("BadStr"),
		}
	}

	pub async fn send_request(&self,  request_type: u8, request_num: u8, value: u16, index: u16, data: &[u8])
	{
		let hdr = hw_decls::DeviceRequest {
			req_type: request_type,
			req_num: request_num,
			value: value,
			index: index,
			length: data.len() as u16,
			};
		let hdr = hdr.to_bytes();
		let sent_len = self.inner.out_only(&hdr, data).await;
		assert_eq!(sent_len, data.len());
	}
}

pub struct BulkEndpointOut
{
	inner: crate::host::Handle<dyn crate::host::BulkEndpointOut>,
}
impl BulkEndpointOut
{
	fn new(host: &Host, addr: u8, ep_num: u8, max_packet_size: usize) -> Self {
		Self {
			inner: host.driver.init_bulk_out(crate::host::EndpointAddr::new(addr, ep_num), max_packet_size),
			}
	}

	pub async fn send(&self, data: &[u8])
	{
		self.inner.send(data).await;
	}
}

pub struct BulkEndpointIn
{
	inner: crate::host::Handle<dyn crate::host::BulkEndpointIn>,
}
impl BulkEndpointIn
{
	fn new(host: &Host, addr: u8, ep_num: u8, max_packet_size: usize) -> Self {
		Self {
			inner: host.driver.init_bulk_in(crate::host::EndpointAddr::new(addr, ep_num), max_packet_size),
			}
	}

	pub async fn recv(&self, data: &mut [u8])
	{
		self.inner.recv(data).await;
	}
}

impl Host
{
	fn add_device<F,A>(&self, make_worker: F)
	where
		F: FnOnce(u8) -> A,
		A: ::core::future::Future<Output=()> + Send + 'static,
	{
		// Allocate address
		match self.addresses.lock().allocate()
		{
		Some(v) => {
			log_notice!("New USB device - alloc address {}", v);
			assert!(v != 0);
			// Create async task for the device
			let cb = Box::pin(make_worker(v));
			// Insert into the worker list for this host
			let mut lh = self.device_workers[v as usize].lock();
			assert!( lh.is_none(), "Address already allocated?" );
			*lh = Some(cb);
			},
		None => log_error!("Out of USB addresses on bus"),
		}
	}

	async fn get_address_zero<'a>(&'a self) -> AddressZeroHandle<'a>
	{
		AddressZeroHandle {
			host: self,
			_lh: self.endpoint_zero_lock.async_lock().await,
			}
	}

	async fn root_event_task(&self)
	{
		loop
		{
			let port_idx = self.driver.async_wait_root().await;
			self.handle_root_event(port_idx);
		}
	}

	fn handle_root_event(&self, port_idx: usize)
	{
		log_debug!("handle_root_event: ({})", port_idx);

		if self.driver.get_port_feature(port_idx, host::PortFeature::CConnection)
		{
			log_trace!("CConnection");
			self.driver.clear_port_feature(port_idx, host::PortFeature::CConnection);
			if self.driver.get_port_feature(port_idx, host::PortFeature::Connection)
			{
				log_debug!("Connection detected, signalling");

				// SAFE: (TODO: unenforced) Requires that `self` is stable in memory
				let hubref = HubRef::Root(unsafe { HostRef::new(self) });
				self.root_ports[port_idx].signal_connected(hubref, port_idx as u8);
			}
			else
			{
				// Was disconnected, need to eliminate all downstream devices
				// - Requires knowing what devices are on this port.
				// - And need to signal to the devices that they've been disconnected
				todo!("Handle port disconnection");
				//self.ports[port_idx].signal_disconnected();
			}
		}
		/*
		else if self.driver.get_port_feature(port_idx, host::PortFeature::CReset)
		{
			self.driver.clear_port_feature(port_idx, host::PortFeature::CReset);
			if self.driver.get_port_feature(port_idx, host::PortFeature::Reset)
			{
			}
			else if self.driver.get_port_feature(port_idx, host::PortFeature::Enable)
			{
				// Allocate an ID, allocate a , send the 'set device ID' request
				//todo!("Push new device to enumeration");
			}
			else
			{
				// Reset complete, but not enabled?
				todo!("Handle port completing reset, but not being enabled?");
			}
		}
		else if self.driver.get_port_feature(port_idx, host::PortFeature::CEnable)
		{
			self.driver.clear_port_feature(port_idx, host::PortFeature::CEnable);
			log_debug!("Change in enable status...");
		}
		*/
		else
		{
			log_trace!("Nothing to do");
		}
	}
}

struct AddressZeroHandle<'a> {
	host: &'a Host,
	_lh: ::kernel::futures::mutex::HeldMutex<'a, ()>,
}
impl<'a> AddressZeroHandle<'a>
{
	async fn send_setup_address(&self, addr: u8) {
		// Send a request with type=0x00, request=5,  value=addr, index=0, and no data
		self.host.endpoint_zero_handle.send_request(0x00, 5, addr as u16, 0, &[]).await
	}
}

impl AddressPool
{
	fn allocate(&mut self) -> Option<u8>
	{
		for i in self.next_id ..= 255 {
			let byte = &mut self.used_ids[i as usize / 8];
			let bitmask = 1 << (i%8);
			if 0 == *byte & bitmask {
				*byte |= bitmask;
				self.next_id = i.checked_add(1).unwrap_or(1);
				return Some(i);
			}
		}
		// Wraparound!
		for i in 1 .. self.next_id {
			let byte = &mut self.used_ids[i as usize / 8];
			let bitmask = 1 << (i%8);
			if 0 == *byte & bitmask {
				*byte |= bitmask;
				self.next_id = i.checked_add(1).unwrap_or(1);
				return Some(i);
			}
		}
		// Exhausted
		None
	}
}

