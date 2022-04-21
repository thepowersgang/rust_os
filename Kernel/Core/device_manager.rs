// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/device_manager.rs
// - Core device manager

use crate::prelude::*;
use crate::sync::Mutex;
use crate::lib::Queue;

module_define!{DeviceManager, [arch], init}

/// A semi-arbitatry integer denoting how well a driver handles a specific device
pub type DriverHandleLevel = u32;

/// IO range binding
pub enum IOBinding
{
	/// Memory-mapped IO space
	Memory(crate::memory::virt::MmioHandle),
	/// x86 IO bus (Base and offset)
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

/// Attrbute on a bus device
#[derive(Debug)]
pub enum AttrValue<'a>
{
	/// Invalid attribute name
	None,
	/// 32-bit integer
	U32(u32),
	/// String value
	String(&'a str),
}
impl<'a> AttrValue<'a> {
	pub fn unwrap_u32(self) -> u32 {
		if let AttrValue::U32(v) = self {
			v
		}
		else {
			panic!("AttrValue::unwrap_u32 - {:?}", self);
		}
	}
	pub fn unwrap_str(self) -> &'a str {
		if let AttrValue::String(v) = self {
			v
		}
		else {
			panic!("AttrValue::unwrap_str - {:?}", self);
		}
	}
}

/// Interface to a device on a bus
pub trait BusDevice:
	Send
{
	fn type_id(&self) -> ::core::any::TypeId;
	/// Returns the device's address on the parent bus
	fn addr(&self) -> u32;
	/// Returns the specified attribute (or 0, if invalid)
	fn get_attr(&self, name: &str) -> AttrValue {
		self.get_attr_idx(name, 0)
	}
	fn get_attr_idx(&self, name: &str, idx: usize) -> AttrValue;
	/// Set the specified attribute
	fn set_attr(&mut self, name: &str, value: AttrValue) {
		self.set_attr_idx(name, 0, value)
	}
	fn set_attr_idx(&mut self, name: &str, idx: usize, value: AttrValue);
	/// Set the power state of this device
	fn set_power(&mut self, state: bool);	// TODO: Power state enum for Off,Standby,Low,On
	/// Bind to the specified IO block (meaning of `block_id` depends on the bus)
	fn bind_io(&mut self, block_id: usize) -> IOBinding {
		self.bind_io_slice(block_id, None)
	}
	fn bind_io_slice(&mut self, block_id: usize, slice: Option<(usize,usize)>) -> IOBinding;
	/// Obtain the specified interrupt vector
	fn get_irq(&mut self, idx: usize) -> u32;
}
impl<'a> dyn BusDevice + 'a
{
	pub fn downcast_ref<T: 'static + BusDevice>(&self) -> Option<&T>
	{
		if self.type_id() == ::core::any::TypeId::of::<T>() {
			// SAFE: The above code has established that the type matches
			Some(unsafe { &*(self as *const _ as *const T) })
		}
		else {
			None
		}
	}
}

/// Abstract driver for a device (creates instances when passed a device)
pub trait Driver:
	Send + Sync
{
	/// Driver's name
	fn name(&self) -> &str;
	/// Bus type the driver binds against (matches value from `BusManager::bus_type`)
	fn bus_type(&self) -> &str;
	/// Return the handling level of this driver for the specified device
	fn handles(&self, bus_dev: &dyn BusDevice) -> DriverHandleLevel;
	/// Requests that the driver bind itself to the specified device
	fn bind(&self, bus_dev: &mut dyn BusDevice) -> Box<dyn DriverInstance+'static>;
}
/// Error type for `Driver::bind`
#[derive(Debug)]
pub enum DriverBindError
{
	OutOfMemory,
	Bug(&'static str),
}
impl_from! {
	From<crate::memory::virt::MapError>(v) for DriverBindError {
		match v
		{
		crate::memory::virt::MapError::OutOfMemory => DriverBindError::OutOfMemory,
		crate::memory::virt::MapError::RangeInUse => DriverBindError::Bug("Memory map range collision"),
		}
	}
}

/// Driver instance (maps directly to a device)
pub trait DriverInstance:
	Send
{
}

/// Internal representation of a device on a bus
struct Device
{
	bus_dev: Box<dyn BusDevice>,
	driver: Option<(Box<dyn DriverInstance>, DriverHandleLevel)>,
	//attribs: Vec<u32>,
}

/// Internal representation of a bus
struct Bus
{
	manager: &'static dyn BusManager,
	devices: Vec<Device>,
}

/// List of registered busses on the system
#[allow(non_upper_case_globals)]
static s_root_busses: Mutex<Queue<Bus>> = mutex_init!(queue_init!());

/// List of registered drivers
#[allow(non_upper_case_globals)]
static s_driver_list: Mutex<Queue<&'static dyn Driver>> = mutex_init!( queue_init!() );

fn init()
{
	// Do nothing!
}

/// Register a bus with the device manager
///
/// Creates a new internal representation of the bus, containg the passed set of devices.
pub fn register_bus(manager: &'static dyn BusManager, devices: Vec<Box<dyn BusDevice>>) //-> BusHandle
{
	let bus = Bus {
		manager: manager,
		// For each device, locate a driver
		devices: devices.into_iter().map(|mut d| Device {
			driver: find_driver(manager, &mut *d),
			//attribs: Vec::new(),
			bus_dev: d,
			}).collect(),
		};
	let mut bus_list_lh = s_root_busses.lock();
	bus_list_lh.push(bus);
	//let ptr: *const _ = bus_list_lh.last().unwrap();
	//BusHandle(ptr)
}

/// Registers a driver with the device manger
pub fn register_driver(driver: &'static dyn Driver)
{
	s_driver_list.lock().push(driver);
	log_debug!("Registering driver {}", driver.name());
	// Iterate known devices and spin up instances if needed
	for bus in s_root_busses.lock().iter_mut()
	{
		log_trace!("bus type {}", bus.manager.bus_type());
		if driver.bus_type() == bus.manager.bus_type()
		{
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
					dev.driver = Some( (driver.bind(&mut *dev.bus_dev), rank) );
				}
			}
		}
	}
}

/**
 * Locate the best registered driver for this device and instanciate it
 */
fn find_driver(bus: &dyn BusManager, bus_dev: &mut dyn BusDevice) -> Option<(Box<dyn DriverInstance>,DriverHandleLevel)>
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

impl IOBinding
{
	/// Returns the x86 IO space base
	#[inline]
	pub fn io_base(&self) -> u16 {
		match *self
		{
		IOBinding::IO(base, _size) => base,
		IOBinding::Memory(_) => panic!("Called IOBinding::io_base on IOBinding::Memory"),
		}
	}
	/// Read a single u8 from the binding
	#[inline]
	pub unsafe fn read_8(&self, ofs: usize) -> u8
	{
		//log_trace!("read_8({:?}, {:#x})", self, ofs);
		match *self
		{
		IOBinding::IO(base, s) => {
			assert!( ofs+1 <= s as usize, "read_u8(IO addr {:#x}+1 > {:#x})", ofs, s );
			crate::arch::x86_io::inb(base + ofs as u16)
			},
		IOBinding::Memory(ref h) => {
			::core::intrinsics::volatile_load( h.as_int_mut::<u8>(ofs) )
			},
		}
	}
	/// Read a single u16 from the binding
	#[inline]
	pub unsafe fn read_16(&self, ofs: usize) -> u16
	{
		//log_trace!("read_16({:?}, {:#x})", self, ofs);
		match *self
		{
		IOBinding::IO(base, s) => {
			assert!( ofs+2 <= s as usize, "read_u16(IO addr {:#x}+2 > {:#x})", ofs, s );
			crate::arch::x86_io::inw(base + ofs as u16)
			},
		IOBinding::Memory(ref h) => {
			::core::intrinsics::volatile_load( h.as_int_mut::<u16>(ofs) )
			},
		}
	}
	/// Read a single u32 from the binding
	#[inline]
	pub unsafe fn read_32(&self, ofs: usize) -> u32
	{
		//log_trace!("read_32({:?}, {:#x})", self, ofs);
		match *self
		{
		IOBinding::IO(base, s) => {
			assert!( ofs+4 <= s as usize, "read_u32(IO addr {:#x}+4 > {:#x})", ofs, s );
			crate::arch::x86_io::inl(base + ofs as u16)
			},
		IOBinding::Memory(ref h) => {
			::core::intrinsics::volatile_load( h.as_int_mut::<u32>(ofs) )
			},
		}
	}
	/// Writes a single u8 to the binding
	#[inline]
	pub unsafe fn write_8(&self, ofs: usize, val: u8)
	{
		//log_trace!("write_8({:?}, {:#x}, {:#02x})", self, ofs, val);
		match *self
		{
		IOBinding::IO(base, s) => {
			assert!( ofs+1 <= s as usize, "write_8(IO addr {:#x}+1 > {:#x})", ofs, s );
			crate::arch::x86_io::outb(base + ofs as u16, val);
			},
		IOBinding::Memory(ref h) => {
			::core::intrinsics::volatile_store( h.as_int_mut::<u8>(ofs), val );
			},
		}
	}
	/// Write a single u32 to the binding
	#[inline]
	pub unsafe fn write_16(&self, ofs: usize, val: u16)
	{
		//log_trace!("write_16({:?}, {:#x}, {:#02x})", self, ofs, val);
		match *self
		{
		IOBinding::IO(base, s) => {
			assert!(ofs+2 <= s as usize, "write_16(IO addr {:#x}+4 > {:#x})", ofs, s);
			crate::arch::x86_io::outw(base + ofs as u16, val);
			},
		IOBinding::Memory(ref h) => {
			::core::intrinsics::volatile_store( h.as_int_mut::<u16>(ofs), val );
			},
		}
	}
	/// Write a single u32 to the binding
	#[inline]
	pub unsafe fn write_32(&self, ofs: usize, val: u32)
	{
		//log_trace!("write_32({:?}, {:#x}, {:#02x})", self, ofs, val);
		match *self
		{
		IOBinding::IO(base, s) => {
			assert!(ofs+4 <= s as usize, "write_32(IO addr {:#x}+4 > {:#x})", ofs, s);
			crate::arch::x86_io::outl(base + ofs as u16, val);
			},
		IOBinding::Memory(ref h) => {
			::core::intrinsics::volatile_store( h.as_int_mut::<u32>(ofs), val );
			},
		}
	}
}

impl_fmt! {
	Display(self, f) for IOBinding {
		match *self
		{
		IOBinding::Memory(ref m) => write!(f, "0x{:08x}", m.phys(), ),
		IOBinding::IO(base, lim) => write!(f, "IO:0x{:04x}+{:x}", base, lim),
		}
	}
}
impl ::core::fmt::Debug for IOBinding
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		match *self
		{
		IOBinding::IO(b, s) => write!(f, "IO({:#x}+{:#x})", b, s),
		IOBinding::Memory(ref h) => write!(f, "Memory({:?})", h),
		}
	}
}

// vim: ft=rust
