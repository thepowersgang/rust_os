// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/device_manager.rs
// - Core device manager

use _common::*;
use sync::Mutex;
use lib::Queue;

module_define!{DeviceManager, [], init}

/// A semi-arbitatry integer denoting how well a driver handles a specific device
pub type DriverHandleLevel = u32;

/// IO range binding
pub enum IOBinding
{
	/// Memory-mapped IO space
	Memory(::memory::virt::AllocHandle),
	/// x86 IO bus
	/// Base and offset
	IO(u16,u16),
}

/// Interface a bus manager instance
pub trait BusManager:
	Send + Sync
{
	/// Returns the textual name of the bus type (e.g. "pci")
	fn bus_type(&self) -> &str;
	/// Returns a list of valid attributes for BusDevice::get_attr
	fn get_attr_names(&self) -> &[&str];
}

/// Interface to a device on a bus
pub trait BusDevice:
	Send
{
	/// Returns the device's address on the parent bus
	fn addr(&self) -> u32;
	/// Retrurns the specified attribute (or 0, if invalid)
	fn get_attr(&self, name: &str) -> u32;
	/// Set the power state of this device
	fn set_power(&mut self, state: bool);	// TODO: Power state enum for Off,Standby,Low,On
	/// Bind to the specified IO block (meaning of `block_id` depends on the bus)
	fn bind_io(&mut self, block_id: usize) -> IOBinding;
}

// TODO: Change this to instead be a structure with a bound Fn reference
// - Structure defines bus type and a set of attribute names/values/masks
/// Abstract driver for a device (creates instances when passed a device)
pub trait Driver:
	Send + Sync
{
	/// Driver's name
	fn name(&self) -> &str;
	/// Bus type the driver binds against (matches value from `BusManager::bus_type`)
	fn bus_type(&self) -> &str;
	/// Return the handling level of this driver for the specified device
	fn handles(&self, bus_dev: &BusDevice) -> DriverHandleLevel;
	/// Requests that the driver bind itself to the specified device
	fn bind(&self, bus_dev: &BusDevice) -> Box<DriverInstance>;
}

/// Driver instance (maps directly to a device)
pub trait DriverInstance:
	Send
{
}

/// Internal representation of a device on a bus
struct Device
{
	bus_dev: Box<BusDevice>,
	driver: Option<(Box<DriverInstance>, DriverHandleLevel)>,
	//attribs: Vec<u32>,
}

struct Bus
{
	manager: &'static BusManager,
	devices: Vec<Device>,
}

/// List of registered busses on the system
#[allow(non_upper_case_globals)]
static s_root_busses: Mutex<Queue<Bus>> = mutex_init!(queue_init!());

/// List of registered drivers
#[allow(non_upper_case_globals)]
static s_driver_list: Mutex<Queue<&'static Driver>> = mutex_init!( queue_init!() );

fn init()
{
	// Do nothing!
}

/// Register a bus with the device manager
///
/// Creates a new internal representation of the bus, containg the passed set of devices.
pub fn register_bus(manager: &'static BusManager, devices: Vec<Box<BusDevice>>)
{
	let bus = Bus {
		manager: manager,
		// For each device, locate a driver
		devices: devices.into_iter().map(|d| Device {
			driver: find_driver(manager, &*d),
			//attribs: Vec::new(),
			bus_dev: d,
			}).collect(),
		};
	s_root_busses.lock().push(bus);
}

/// Registers a driver with the device manger
pub fn register_driver(driver: &'static Driver)
{
	s_driver_list.lock().push(driver);
	log_debug!("Registering driver {}", driver.name());
	// Iterate known devices and spin up instances if needed
	for bus in s_root_busses.lock().iter_mut()
	{
		log_trace!("bus type {}", bus.manager.bus_type());
		for dev in bus.devices.iter_mut()
		{
			let rank = driver.handles(&*dev.bus_dev);
			log_debug!("rank = {:?}", rank);
			if rank == 0
			{
				// SKIP!
			}
			else if dev.driver.is_some()
			{
				let bind = dev.driver.as_ref().unwrap();
				let cur_rank = bind.1;
				if cur_rank > rank
				{
					// Existing driver is better
				}
				else if cur_rank == rank
				{
					// Fight!
				}
				else
				{
					// New driver is better
					panic!("TODO: Unbind driver and bind in new one");
				}
			}
			else
			{
				// Bind new driver
				dev.driver = Some( (driver.bind(&*dev.bus_dev), rank) );
			}
		}
	}
}

/**
 * Locate the best registered driver for this device and instanciate it
 */
fn find_driver(bus: &BusManager, bus_dev: &BusDevice) -> Option<(Box<DriverInstance>,DriverHandleLevel)>
{
	log_debug!("Finding driver for {}:{:x}", bus.bus_type(), bus_dev.addr());
	let mut best_ranking = 0;
	let mut best_driver = None;
	for driver in s_driver_list.lock().iter()
	{
		if bus.bus_type() == driver.bus_type()
		{
			let ranking = driver.handles(bus_dev);
			if ranking == 0
			{
				// Doesn't handle this device
			}
			else if ranking > best_ranking
			{
				// Best so far
				best_driver = Some( *driver );
				best_ranking = ranking;
			}
			else if ranking == best_ranking
			{
				// A tie, this is not very good
				//log_warning!("Tie for device {}:{:x} between {} and {}",
				//	bus.bus_type(), bus_dev.addr(), driver, best_driver.unwrap());
			}
			else
			{
				// Not as good as current, move along
			}
		}
	}
	best_driver.map(|d| (d.bind(bus_dev), best_ranking))
}

//impl<'a> ::core::fmt::Show for BusDevice+'a
//{
//	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> Result<(),::core::fmt::Error> {
//		write!(f, "Dev {}:{:x}", "TODO", self.addr())
//	}
//}

// vim: ft=rust
