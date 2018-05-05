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

enum Meta
{
	RootHub(Aref<Host>),
	Hub(HubDevice),
}

struct Host
{
	driver: Box<host::HostController>,
	next_id: u8,
	used_ids: [u8; 128/8],

	//// If true, EP0 is currently being enumerated
	//endpoint_zero_state: bool,
}
struct HubDevice
{
	host: ArefBorrow<Host>,
	int_ep: host::Handle<host::InterruptEndpoint>,
}
///// An endpoint currently being enumerated
//struct EnumeratingDevice
//{
//	obj: async::Object,
//}

static WATCH_LIST: ::kernel::sync::Mutex<Vec<Meta>> = ::kernel::sync::Mutex::new(Vec::new_const());
static EVENT_QUEUE: ::kernel::sync::Queue<(usize, usize)> = ::kernel::sync::Queue::new_const();
//static ENUM_ENDPOINTS: ::kernel::sync::Mutex<Vec<Box<Endpoint>>> = ::kernel::sync::Mutex::new(Vec::new_const());

pub fn register_host(mut h: Box<host::HostController>)
{
	let mut lh = WATCH_LIST.lock();
	let idx = lh.len();
	// Note: Setting the root waiter should trigger event pushes for any connected port
	// - This doesn't race, because the list lock is still held.
	h.set_root_waiter(&EVENT_QUEUE, idx);
	lh.push(Meta::RootHub(Aref::new(Host {
		driver: h,
		next_id: 1,
		used_ids: [0; 128/8],
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
		Meta::RootHub(ref h) => h.handle_root_event(data),
		Meta::Hub(ref h) => h.handle_int(data),
		}
	}
}
//fn enum_worker_thread()
//{
//	loop
//	{
//		// 1. Register sleeps on new endpoints
//		// - Get device descriptor
//		// - Enumerate available configurations
//	}
//}

impl Host
{
	fn handle_root_event(&self, port_idx: usize)
	{
		log_debug!("handle_root_event: ({})", port_idx);
		// TODO: Support timing port updates by using large values of `port_idx`
		if self.driver.get_port_feature(port_idx, host::PortFeature::CConnection)
		{
			self.driver.clear_port_feature(port_idx, host::PortFeature::CConnection);
			if self.driver.get_port_feature(port_idx, host::PortFeature::Connection)
			{
				// Connection detected, either:
				// - Trigger a reset (and record this port as the next EP0)
				// - OR, add to a list of to-be-enumerated ports (if EP0 is already in use)
				if self.driver.get_port_feature(port_idx, host::PortFeature::Power)
				{
					//if self.endpoint_zero_in_use {
					//}
					//else {
					//	self.endpoint_zero_in_use = true;
						self.driver.set_port_feature(port_idx, host::PortFeature::Reset);
					//}
				}
				else
				{
					todo!("Power on a newly connected port");
				}
			}
			else
			{
				// Was disconnected, need to eliminate all downstream devices
				// - Requires knowing what devices are on this port.
				todo!("Handle port disconnection");
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

impl HubDevice
{
	fn handle_int(&self, _size: usize)
	{
		let data_handle = self.int_ep.get_data();
		let data = data_handle.get();
		todo!("Process interrupt bytes from host - {:?}", ::kernel::logging::HexDump(data));
	}
}


