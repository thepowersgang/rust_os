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

	//endpoint_zero_state: (),
}
struct HubDevice
{
	host: ArefBorrow<Host>,
	int_ep: host::Handle<host::InterruptEndpoint>,
}

static WATCH_LIST: ::kernel::sync::Mutex<Vec<Meta>> = ::kernel::sync::Mutex::new(Vec::new_const());
static EVENT_QUEUE: ::kernel::sync::Queue<(usize, usize)> = ::kernel::sync::Queue::new_const();

pub fn register_host(mut h: Box<host::HostController>)
{
	let mut lh = WATCH_LIST.lock();
	let idx = lh.len();
	h.set_root_waiter(&EVENT_QUEUE, idx);
	lh.push(Meta::RootHub(Aref::new(Host {
		driver: h,
		next_id: 1,
		used_ids: [0; 128/8],
		})));
	// TODO: Wake the worker and get it to check the root hub.
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

impl Host
{
	fn handle_root_event(&self, port_idx: usize)
	{
		// TODO: Support timing port updates by using large values of `port_idx`
		if self.driver.get_port_feature(port_idx, host::PortFeature::CConnection)
		{
			self.driver.clear_port_feature(port_idx, host::PortFeature::CConnection);
			if self.driver.get_port_feature(port_idx, host::PortFeature::Connection)
			{
				// Connection detected, either:
				// - Trigger a reset (and record this port as the next EP0)
				// - OR, add to a list of to-be-enumerated ports (if EP0 is already in use)
				self.driver.set_port_feature(port_idx, host::PortFeature::Reset);
			}
			else
			{
				// Was disconnected, need to eliminate all downstream devices
				// - Requires knowing what devices are on this port.
				todo!("Handle port disconnection");
			}
		}
		else if self.driver.get_port_feature(port_idx, host::PortFeature::CEnable)
		{
			self.driver.clear_port_feature(port_idx, host::PortFeature::CEnable);
			if self.driver.get_port_feature(port_idx, host::PortFeature::Enable)
			{
				// Hand over to EP0 enumeration.
				todo!("Push new device to enumeration");
			}
			else
			{
				todo!("Handle port being disabled?");
			}
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


