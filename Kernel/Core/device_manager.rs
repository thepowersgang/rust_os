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
	fn get_attr_names() -> &[&str];
}

pub trait BusDevice
{
	fn get_attr(name: &str) -> u32;
	fn set_power(state: bool);	// TODO: Power state enum for Off,Standby,Low,On
	fn bind_io(block_id: uint) -> IOBinding;
}

pub trait Driver
{
	
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

//#[allow(non_upper_case_globals)]
//static s_driver_list: Mutex<Vec<&'static Driver>> = mutex_init!( Vec::new() );

fn init()
{
	// Do nothing!
}

pub fn register_bus(manager: &'static BusManager+'static, devices: Vec<Box<BusDevice+'static>>)
{
	let bus = Bus {
		manager: manager,
		devices: devices.into_iter().map(|d| Device {
			driver: find_driver(&*d),
			bus_dev: d,
			attribs: Vec::new(),
			}).collect(),
		};
	s_root_busses.lock().push(bus);
}

pub fn register_driver(driver: &'static Driver+'static)
{
	
}

fn find_driver(bus_dev: &BusDevice) -> Option<Box<DriverInstance+'static>>
{
	/*
	for driver in s_driver_list.lock().items()
	{
		if driver.handles(bus_dev) {
			return Some( driver.bind(bus_dev) );
		}
	}
	*/
	None
}

// vim: ft=rust
