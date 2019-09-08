// 
//! USB Core
#![no_std]
#![feature(linkage)]	// for module_define!
use kernel::prelude::*;
use kernel::lib::mem::aref::{Aref,ArefBorrow};

#[macro_use]
extern crate kernel;
extern crate stack_dst;

module_define!{usb_core, [], init}

fn init()
{
	// Start the worker, then forget the handle
	::core::mem::forget( ::kernel::threads::WorkerThread::new("USB Hubs", worker_thread) );
}

mod hub;
pub mod host;
pub mod handle;

enum Hub
{
	Root(Aref<Host>),
	Device(HubDevice),
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
	addresses: ::kernel::sync::Mutex<AddressPool>,

	//// If true, EP0 is currently being enumerated
	//endpoint_zero_state: bool,
	//ports: Vec<Port>,
}
struct HubDevice
{
	host: ArefBorrow<Host>,
	int_ep: host::Handle<dyn host::InterruptEndpoint>,
}
///// An endpoint currently being enumerated
//struct EnumeratingDevice
//{
//	obj: async::Object,
//}

static WATCH_LIST: ::kernel::sync::Mutex<Vec<Hub>> = ::kernel::sync::Mutex::new(Vec::new_const());
static EVENT_QUEUE: ::kernel::sync::Queue<(usize, usize)> = ::kernel::sync::Queue::new_const();
///// A list of known devices
//static ENUM_DEVICE_LIST: ::kernel::sync::Mutex<Vec< <typeof Port::worker as FnOnce>::Output >> = :kernel::sync::Mutex::new(Vec::new_const());
//static ENUM_DEVICE_LIST: ::kernel::sync::Mutex<Vec<PortTask>> = :kernel::sync::Mutex::new(Vec::new_const());

/// Add a new host controller/bus to the system
pub fn register_host(mut h: Box<dyn host::HostController>)
{
	let mut lh = WATCH_LIST.lock();
	let idx = lh.len();
	// Note: Setting the root waiter should trigger event pushes for any connected port
	// - This doesn't race, because the list lock is still held.
	h.set_root_waiter(&EVENT_QUEUE, idx);
	lh.push(Hub::Root(Aref::new(Host {
		driver: h,
		addresses: ::kernel::sync::Mutex::new(AddressPool {
			next_id: 1,
			used_ids: [0; 128/8],
			}),
		})));
}

fn worker_thread()
{
	loop
	{
		// Wait on a queue of (usize, usize) where the first is allocated by this code, and the second is from from the HCD
		let (idx, data) = EVENT_QUEUE.wait_pop();
		// This needs to check:
		// - Root hub changes (when signalled by the HCD)
		// - Interrupt reponses from hub devices
		let lh = WATCH_LIST.lock();
		match lh[idx]
		{
		Hub::Root  (ref h) => h.handle_root_event(data),
		Hub::Device(ref h) => h.handle_int(data),
		}
	}
}
#[allow(dead_code)]	// TODO: Remove when used
impl Hub
{
	fn set_port_feature(&self, port_idx: usize, feat: host::PortFeature)
	{
		match self
		{
		&Hub::Root  (ref h) => h.driver.set_port_feature(port_idx, feat),
		&Hub::Device(ref h) => h.set_port_feature(port_idx, feat),
		}
	}
	fn clear_port_feature(&self, port_idx: usize, feat: host::PortFeature)
	{
		match self
		{
		&Hub::Root  (ref h) => h.driver.clear_port_feature(port_idx, feat),
		&Hub::Device(ref h) => h.clear_port_feature(port_idx, feat),
		}
	}
	fn get_port_feature(&self, port_idx: usize, feat: host::PortFeature) -> bool
	{
		match self
		{
		&Hub::Root  (ref h) => h.driver.get_port_feature(port_idx, feat),
		&Hub::Device(ref h) => h.get_port_feature(port_idx, feat),
		}
	}
}

struct Port
{
	//connection_signaller: kernel::r#async::Signaller,
	//hub: /*reference to the hub*/,
}
impl Port
{
	//fn set_port_feature(&self, feat: host::PortFeature) {
	//	self.hub.set_port_feature(self.port_idx, feat)
	//}
	//fn clear_port_feature(&self, feat: host::PortFeature) {
	//	self.hub.clear_port_feature(self.port_idx, feat)
	//}
	//fn get_port_feature(&self, feat: host::PortFeature) -> bool {
	//	self.hub.get_port_feature(self.port_idx, feat)
	//}
	//fn getclear_port_feature(&self, feat: host::PortFeature) -> bool {
	//	let rv = self.get_port_feature(feat)
	//	if rv {
	//		self.clear_port_feature(feat);
	//	}
	//	rv
	//}

	fn signal_connected(&self)
	{
		//self.connection_signaller.signal();
	}

	#[cfg(false_)]
	async fn initialise_port(&self) -> usize
	{
		let addr0_handle = r#await!( self.host().get_address_zero() );
		if ! self.get_port_feature(host::PortFeature::Power)
		{
			todo!("Power on a newly connected port");
		}
		self.set_port_feature(host::PortFeature::Reset);
		r#await!( ::kernel::futures::msleep(50) );
		self.clear_port_feature(host::PortFeature::Reset);
		r#await!( ::kernel::futures::msleep(2) );
		self.clear_port_feature(host::PortFeature::Enable);
		let address = self.host().allocate_address();
		r#await!( addr0_handle.send_setup_address(address) );
		address
	}

	#[cfg(false_)]
	async fn worker(&self)
	{
		loop
		{
			r#await!( self.connection_signaller.async_wait() );
			let addr = r#await!( self.initialise_port() );
			
			let ep0 = self.make_control_endpoint(/*ep_num=*/0, /*max_packet_size=*/64);
			// Enumerate device
			let dev_descr: hw::DeviceDescriptor = r#await!( ep0.read_descriptor(/*index*/0) );
			log_debug!("dev_descr = {:?}", dev_descr);
			log_debug!("dev_descr.manufacturer_str = {}", r#await!(ep0.read_string(dev_descr.manufacturer_str)));
			log_debug!("dev_descr.product_str = {}", r#await!(ep0.read_string(dev_descr.product_str)));
			log_debug!("dev_descr.serial_number_str = {}", r#await!(ep0.read_string(dev_descr.serial_number_str)));
		}
	}
}

impl Host
{
	#[allow(dead_code)] // TODO: Remove when used
	fn allocate_address(&self) -> Option<u8>
	{
		match self.addresses.lock().allocate()
		{
		Some(v) => {
			assert!(v != 0);
			Some(v)
			},
		None => None,
		}
	}
	#[cfg(false_)]
	async fn get_address_zero(&self) -> EnumDeviceHandle
	{
		()
	}

	fn handle_root_event(&self, port_idx: usize)
	{
		log_debug!("handle_root_event: ({})", port_idx);
		// TODO: Support timing port updates by using large values of `port_idx`?

		if self.driver.get_port_feature(port_idx, host::PortFeature::CConnection)
		{
			self.driver.clear_port_feature(port_idx, host::PortFeature::CConnection);
			if self.driver.get_port_feature(port_idx, host::PortFeature::Connection)
			{
				//self.ports[port_idx].signal_connected();
				// Ensure that the port is on the active list
			}
			else
			{
				// Was disconnected, need to eliminate all downstream devices
				// - Requires knowing what devices are on this port.
				todo!("Handle port disconnection");
				//self.ports[port_idx].signal_disconnected();
			}
		}
		else if self.driver.get_port_feature(port_idx, host::PortFeature::CReset)
		{
			self.driver.clear_port_feature(port_idx, host::PortFeature::CReset);
			if self.driver.get_port_feature(port_idx, host::PortFeature::Reset)
			{
			}
			else if self.driver.get_port_feature(port_idx, host::PortFeature::Enable)
			{
				// Allocate an ID, allocate a , send the 'set device ID' request
				todo!("Push new device to enumeration");
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
		else
		{
		}
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

impl HubDevice
{
	fn handle_int(&self, _size: usize)
	{
		let data_handle = self.int_ep.get_data();
		let data = data_handle.get();
		todo!("Process interrupt bytes from host - {:?}", ::kernel::logging::HexDump(data));
	}

	fn set_port_feature(&self, port_idx: usize, feat: host::PortFeature) {
		todo!("HubDevice::set_port_feature({}, {:?})", port_idx, feat)
	}
	fn clear_port_feature(&self, port_idx: usize, feat: host::PortFeature) {
		todo!("HubDevice::clear_port_feature({}, {:?})", port_idx, feat)
	}
	fn get_port_feature(&self, port_idx: usize, feat: host::PortFeature) -> bool {
		todo!("HubDevice::get_port_feature({}, {:?})", port_idx, feat)
	}

}


