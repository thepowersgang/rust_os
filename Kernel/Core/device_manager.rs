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

trait BusManager
{
}

pub trait BusDevice
{
	fn get_attr(name: &str) -> u32;
	fn bind_io(block_id: uint) -> IOBinding;
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

static s_root_busses: Mutex<Queue<Bus>> = mutex_init!(queue_init!());

fn init()
{
	// Do nothing!
}

//pub fn register_bus(devices: Vec<Box<BusDevice+'static>>)
//{
//}

// vim: ft=rust
