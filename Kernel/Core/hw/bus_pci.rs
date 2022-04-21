// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/pci.rs
// - PCI Bus Handling
use crate::prelude::*;
use crate::device_manager::BusDevice;
use crate::lib::mem::aref::ArefBorrow;

const MAX_FUNC: u8 = 8;	// Address restriction
const MAX_DEV: u8 = 32;	// Address restriction
const CONFIG_WORD_IDENT: u8 = 0;
const CONFIG_WORD_CLASS: u8 = 2;

struct PCIDev
{
	interface: ArefBorrow<dyn PciInterface>,
	addr: u16,	// Bus,Slot,Fcn
	vendor: u16,
	device: u16,
	class: u32,

	// TODO: Include bound status, and BAR mappings
	config: [u32; 16],
}

enum BAR
{
	None,
	IO(u16, u16),	// base, size
	Mem(u64,u32,bool),	// Base, size, prefetchable
}

struct PCIBusManager;
struct PCIChildBusDriver;

#[allow(non_upper_case_globals)]
static s_pci_bus_manager: PCIBusManager = PCIBusManager;
#[allow(non_upper_case_globals)]
static s_pci_child_bus_driver: PCIChildBusDriver = PCIChildBusDriver;
static S_ATTR_NAMES: [&'static str; 3] = ["vendor", "device", "class"];

module_define!{PCI, [DeviceManager], init}

pub trait PciInterface: Send + Sync
{
	/// Read a word from the PCI config space
	fn read_word(&self, bus_addr: u16, word_idx: u8) -> u32;
	/// UNSAFE: Writing to the PCI config space can do strange things
	unsafe fn write_word(&self, bus_addr: u16, word_idx: u8, val: u32);

	/// Thread safe process or:
	/// - Read previous value
	/// - Write the provided mask
	/// - Read new value
	/// - Restore original value
	/// Used to get the changable bits from a BAR
	///
	/// Returns (`original`, `masked`)
	///
	/// UNSAFE: Writing to the PCI config space can do strange things
	unsafe fn get_mask(&self, bus_addr: u16, word_idx: u8, in_mask: u32) -> (u32, u32);
}

pub fn register_bus(interface: ArefBorrow<dyn PciInterface>)
{
	let devs = scan_bus(&interface, 0);
	crate::device_manager::register_bus(&s_pci_bus_manager, devs);
}

fn init()
{
	crate::device_manager::register_driver(&s_pci_child_bus_driver);
	
	// - All drivers that have PCI bindings should be waiting on this to load
}

impl crate::device_manager::BusManager for PCIBusManager
{
	fn bus_type(&self) -> &str { "pci" }
	fn get_attr_names(&self) -> &[&str]
	{
		&S_ATTR_NAMES
	}
}

impl crate::device_manager::Driver for PCIChildBusDriver
{
	fn name(&self) -> &str {
		"bus-pci"
	}
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &dyn crate::device_manager::BusDevice) -> u32
	{
	let d = bus_dev.downcast_ref::<PCIDev>().expect("Not a PCI dev?");
		let bridge_type = (d.config[3] >> 16) & 0x7F;
		// 0x00 == Normal device, 0x01 = PCI-PCI Bridge
		// -> There should only be one PCI bridge handler, but bind low just in case
		if bridge_type == 0x01 { 1 } else { 0 }
	}
	fn bind(&self, bus_dev: &mut dyn crate::device_manager::BusDevice) -> Box<dyn (crate::device_manager::DriverInstance)>
	{
		let d = bus_dev.downcast_ref::<PCIDev>().expect("Not a PCI dev?");
		let bridge_type = (d.config[3] >> 16) & 0x7F;
		assert!(bridge_type == 0x01, "PCIChildBusDriver::bind on a device were `handles` should have failed");
		// Get sub-bus number
		let sec_bus_id = (d.config[6] >> 8) & 0xFF;
		log_debug!("PCI Bridge Bind: sec_bus_id = {:#02x}", sec_bus_id);
		
		todo!("PCIChildBusDriver::bind");
	}
}

impl crate::device_manager::BusDevice for PCIDev
{
	fn type_id(&self) -> ::core::any::TypeId {
		::core::any::TypeId::of::<Self>()
	}
	fn addr(&self) -> u32 {
		self.addr as u32
	}
	fn get_attr_idx(&self, name: &str, idx: usize) -> crate::device_manager::AttrValue {
		use crate::device_manager::AttrValue;
		match name
		{
		"vendor" => AttrValue::U32(self.vendor as u32),
		"device" => AttrValue::U32(self.device as u32),
		"class" => AttrValue::U32(self.class),
		"bus_master" => AttrValue::U32(if self.config[1] & 4 == 0 { 0 } else { 1 }),
		"raw_config" => {
			if idx >= 256 || idx % 4 != 0 {
				AttrValue::None
			}
			else {
				AttrValue::U32(self.interface.read_word(self.addr, idx as u8 / 4))
			}
			},
		_ => {
			log_warning!("Request for non-existant attr '{}' on device 0x{:05x}", name, self.addr);
			AttrValue::None
			},
		}
	}
	fn set_attr_idx(&mut self, name: &str, _idx: usize, value: crate::device_manager::AttrValue) {
		use crate::device_manager::AttrValue;
		match (name,value)
		{
		("vendor", _)|
		("device", _)|
		("class", _) => {
			log_warning!("Attempting to set read-only attr '{}' on device {:#05x}", name, self.addr);
			},
		// Enable/Disable PCI bus-mastering support
		("bus_master", AttrValue::U32(value)) => {
			if value != 0 {
				self.config[1] |= 4;
			}
			else {
				self.config[1] &= !4;
			}
			// SAFE: This is just changing the bus-mastering bit (which can't - on its own - cause unsafety)
			unsafe {
				self.interface.write_word(self.addr, 1, self.config[1]);
			}
			},
		_ => {
			log_warning!("Attempting to set non-existant attr '{}' on device 0x{:05x}", name, self.addr);
			},
		}
	}
	fn set_power(&mut self, state: bool)
	{
		// Nope
		todo!("Set power state of PCI devices (state={})", state);
	}
	fn bind_io_slice(&mut self, block_id: usize, slice: Option<(usize,usize)>) -> crate::device_manager::IOBinding
	{
		if block_id > 6 {
			panic!("PCI bind_io - block_id out of range (max 5, got {})", block_id);
		}
		if block_id % 1 == 1 {
			if self.config[4+block_id-1] & 7 == 4 {
				// Accessing the second word of a 64-bit BAR, this is an error.
				panic!("PCI bind_io - Requesting second word of a 64-bit BAR");
			}
		}
		// TODO: Ensure that the BAR isn't already bound

		match parse_bar(&*self.interface, self.addr, 4+block_id as u8)
		{
		BAR::None => {
			log_error!("PCI bind_io - Request for BAR{} of {:#x} which isn't populated", block_id, self.addr);
			crate::device_manager::IOBinding::IO(0,0)
			},
		BAR::IO(b,s) => {
			if let Some(slice) = slice {
				if slice.0 >= s as usize || slice.1 + slice.0 > s as usize {
					crate::device_manager::IOBinding::IO(0,0)
				}
				else {
					crate::device_manager::IOBinding::IO(b + slice.0 as u16, slice.1 as u16)
				}
			}
			else {
				crate::device_manager::IOBinding::IO(b,s)
			}
			},
		BAR::Mem(base, size, _prefetchable) => {
			let (base, size) = if let Some(slice) = slice {
					if slice.0 >= size as usize || slice.1 + slice.0 > size as usize {
						return crate::device_manager::IOBinding::IO(0,0);
					}
					(base + slice.0 as u64, slice.1 as u32)
				}
				else {
					(base, size)
				};
			// TODO: Ensure safety by preventing multiple bindings to a BAR
			// Assume SAFE: Shouldn't be aliased
			let ah = unsafe { crate::memory::virt::map_mmio(base as crate::memory::PAddr, size as usize).unwrap() };
			crate::device_manager::IOBinding::Memory( ah )
			}
		}
	}
	fn get_irq(&mut self, idx: usize) -> u32
	{
		if idx == 0
		{
			self.config[0x3C/4] & 0xFF
		}
		else
		{
			todo!("PCI get_irq {} > 0", idx);
		}
	}
}

fn scan_bus(interface: &ArefBorrow<dyn PciInterface>, bus_id: u8) -> Vec<Box<dyn BusDevice+'static>>
{
	log_trace!("PCI scan_bus({})", bus_id);
	let mut ret: Vec<Box<dyn BusDevice>> = Vec::new();
	for devidx in 0 .. MAX_DEV
	{
		match get_device(interface, bus_id, devidx, 0)
		{
		Some(devinfo) => {
			let is_multifunc = (devinfo.config[3] & 0x0080_0000) != 0;
			log_debug!("{:?}", devinfo);
			// Increase device count
			ret.push(box devinfo);
			// Handle multi-function devices (iterate from 1 onwards)
			if is_multifunc
			{
				for fcnidx in 1 .. MAX_FUNC
				{
					if let Some(devinfo) = get_device(interface, bus_id, devidx, fcnidx)
					{
						log_debug!("{:?}", devinfo);
						ret.push(box devinfo);
					}
				}
			}
			},
		None => {
			// Move along, nothing to see here
			},
		}
	}
	ret
}

fn get_device(int: &ArefBorrow<dyn PciInterface>, bus_id: u8, devidx: u8, function: u8) -> Option<PCIDev>
{
	let addr = get_pci_addr(bus_id, devidx, function);
	let idword = int.read_word(addr, CONFIG_WORD_IDENT);
	
	if idword & 0xFFFF == 0xFFFF {
		None
	}
	else {
		Some(PCIDev {
			addr: addr,
			vendor: (idword & 0xFFFF) as u16,
			device: (idword >> 16) as u16,
			class: int.read_word(addr, CONFIG_WORD_CLASS),
			config: [
				idword                , int.read_word(addr, 1),
				int.read_word(addr, 2), int.read_word(addr, 3),
				int.read_word(addr, 4), int.read_word(addr, 5),
				int.read_word(addr, 6), int.read_word(addr, 7),
				int.read_word(addr, 8), int.read_word(addr, 9),
				int.read_word(addr,10), int.read_word(addr,11),
				int.read_word(addr,12), int.read_word(addr,13),
				int.read_word(addr,14), int.read_word(addr,15),
				],
			// TODO: Parse all BARs here too?
			interface: int.clone(),
			})
	}
}

fn parse_bar(int: &dyn PciInterface, addr: u16, word: u8) -> BAR
{
	assert!(word >= 4);
	assert!(word-4 < 6);
	let value = int.read_word(addr, word);
	log_trace!("parse_bar({}) value={:#x}", word-4, value);
	if value == 0
	{
		log_debug!("parse_bar: None");
		BAR::None
	}
	else if value & 1 == 0
	{
		// SAFE: Accessing a validated BAR slot
		let (_, one_value) = unsafe { int.get_mask(addr, word, !0u32) };
		let size = !(one_value & 0xFFFF_FFF0) + 1;
		log_debug!("parse_bar: (memory) one_value={:#x}, size={:#x}, value={:#x}", one_value, size, value);
		// memory BAR
		let pf = (value >> 3) & 1;
		let ty = (value >> 1) & 3;
		match ty
		{
		0 => BAR::Mem(value as u64 & !0xF, size, pf == 1),	// 32-bit
		1 => BAR::None,	// reserved
		2 => {	// 64-bit
			assert!(word % 2 == 0);
			// SAFE: Accessing a validated BAR slot
			let (value2, one_value2) = unsafe { int.get_mask(addr, word+1, !0u32) };
			let size2 = !one_value2;	// No+1 needed? (What if size==0?)
			assert!(size2 == 0, "TODO: Support 64-bit BARs with sizes >4GB - size={},size2={}", size, size2);
			let addr = (value2 as u64) << 32 | (value as u64 & !0xF);
			log_debug!("parse_bar: (memory 64) addr={:#x} size={:#x}", addr, size);
			
			BAR::Mem( addr, size, pf == 1 )
			},
		3 => BAR::None,	// reserved
		_ => unreachable!()
		}
	}
	else
	{
		// IO BAR
		// SAFE: Accessing a validated BAR slot
		let (_, one_value) = unsafe { int.get_mask(addr, word, 0xFFFF) };
		let size = ( !(one_value & 0xFFFC) + 1 ) & 0xFFFF;
		log_debug!("parse_bar: (IO) one_value = {:#x}, size={:#x}, value={:#x}", one_value, size, value);
		BAR::IO( (value & 0xFFFC) as u16, size as u16 )
	}
}

fn get_pci_addr(bus_id: u8, dev: u8, fcn: u8) -> u16
{
	assert!(dev < MAX_DEV);
	assert!(fcn < MAX_FUNC);
	((bus_id as u16) << 8) | ((dev as u16) << 3) | (fcn as u16)
}

impl ::core::fmt::Debug for PCIDev
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::result::Result<(),::core::fmt::Error>
	{
		write!(f, "{:#04x} Ven:{:04x} Dev:{:04x} Class {:08x} Hdr={:02x}", self.addr, self.vendor, self.device, self.class, (self.config[3] >> 16) & 0xFF)
	}
}

// vim: ft=rust
