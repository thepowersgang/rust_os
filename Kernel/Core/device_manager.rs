// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/device_manager.rs
// - Core device manager

use _common::*;
use sync::Mutex;
use lib::Queue;

module_define!(DeviceManager, [], init)

pub enum IOBinding
{
	IOBindMemory(::memory::virt::AllocHandle),
	IOBindIO(u16,u16),
}

pub trait BusManager
{
	fn bus_type(&self) -> &str;
	fn get_attr_names(&self) -> &[&str];
}

pub trait BusDevice// : ::core::fmt::Show
{
	fn addr(&self) -> u32;
	fn get_attr(&self, name: &str) -> u32;
	fn set_power(&mut self, state: bool);	// TODO: Power state enum for Off,Standby,Low,On
	fn bind_io(&mut self, block_id: uint) -> IOBinding;
}

pub trait Driver
{
	fn bus_type(&self) -> &str;
	fn handles(&self, bus_dev: &BusDevice) -> bool;
	fn bind(&self, bus_dev: &BusDevice) -> Box<DriverInstance+'static>;
}

pub trait DriverInstance
{
}

struct Device
{
	bus_dev: Box<BusDevice+'static>,
	driver: Option<Box<DriverInstance+'static>>,
	attribs: Vec<u32>,
}

struct Bus
{
	manager: &'static BusManager+'static,
	devices: Vec<Device>,
}

#[allow(non_upper_case_globals)]
static s_root_busses: Mutex<Queue<Bus>> = mutex_init!(queue_init!());

#[allow(non_upper_case_globals)]
static s_driver_list: Mutex<Queue<&'static Driver+'static>> = mutex_init!( queue_init!() );

fn init()
{
	// Do nothing!
}

pub fn register_bus(manager: &'static BusManager+'static, devices: Vec<Box<BusDevice+'static>>)
{
	let bus = Bus {
		manager: manager,
		devices: devices.into_iter().map(|d| Device {
			driver: find_driver(manager, &*d),
			attribs: Vec::new(),
			bus_dev: d,
			}).collect(),
		};
	s_root_busses.lock().push(bus);
}

pub fn register_driver(driver: &'static Driver+'static)
{
	s_driver_list.lock().push(driver);
	// TODO: Iterate known devices and spin up instances if needed
}

fn find_driver(bus: &BusManager, bus_dev: &BusDevice) -> Option<Box<DriverInstance+'static>>
{
	log_debug!("Finding driver for {}:{:x}", bus.bus_type(), bus_dev.addr());
	for driver in s_driver_list.lock().items()
	{
		if bus.bus_type() == driver.bus_type() && driver.handles(bus_dev) {
			return Some( driver.bind(bus_dev) );
		}
	}
	None
}

//impl<'a> ::core::fmt::Show for BusDevice+'a
//{
//	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> Result<(),::core::fmt::Error> {
//		write!(f, "Dev {}:{:x}", "TODO", self.addr())
//	}
//}

// vim: ft=rust
