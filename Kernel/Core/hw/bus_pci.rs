// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/pci.rs
// - PCI Bus Handling
use prelude::*;

use device_manager::BusDevice;

const MAX_FUNC: u8 = 8;	// Address restriction
const MAX_DEV: u8 = 32;	// Address restriction
const CONFIG_WORD_IDENT: u8 = 0;
const CONFIG_WORD_CLASS: u8 = 2;

struct PCIDev
{
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

fn init()
{
	::device_manager::register_driver(&s_pci_child_bus_driver);
	
	// 1. Enumerate PCI bus(es)
	let devs = scan_bus(0);
	//log_debug!("devs = {}", devs);
	::device_manager::register_bus(&s_pci_bus_manager, devs);
	// - All drivers that have PCI bindings should be waiting on this to load
}

impl ::device_manager::BusManager for PCIBusManager
{
	fn bus_type(&self) -> &str { "pci" }
	fn get_attr_names(&self) -> &[&str]
	{
		&S_ATTR_NAMES
	}
}

impl ::device_manager::Driver for PCIChildBusDriver
{
	fn name(&self) -> &str {
		"bus-pci"
	}
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &::device_manager::BusDevice) -> u32
	{
		let addr = bus_dev.addr() as u16;
		let bridge_type = (read_word(addr, 3) >> 16) & 0x7F;
		// 0x00 == Normal device, 0x01 = PCI-PCI Bridge
		// -> There should only be one PCI bridge handler, but bind low just in case
		if bridge_type == 0x01 { 1 } else { 0 }
	}
	fn bind(&self, bus_dev: &mut ::device_manager::BusDevice) -> Box<::device_manager::DriverInstance+'static>
	{
		let addr = bus_dev.addr() as u16;
		let bridge_type = (read_word(addr, 3) >> 16) & 0x7F;
		assert!(bridge_type == 0x01);
		// Get sub-bus number
		let sec_bus_id = (read_word(addr, 6) >> 8) & 0xFF;
		log_debug!("PCI Bridge Bind: sec_bus_id = {:#02x}", sec_bus_id);
		
		todo!("PCIChildBusDriver::bind");
	}
}

impl ::device_manager::BusDevice for PCIDev
{
	fn addr(&self) -> u32 {
		self.addr as u32
	}
	fn get_attr(&self, name: &str) -> ::device_manager::AttrValue {
		use device_manager::AttrValue;
		match name
		{
		"vendor" => AttrValue::U32(self.vendor as u32),
		"device" => AttrValue::U32(self.device as u32),
		"class" => AttrValue::U32(self.class),
		"bus_master" => AttrValue::U32(if self.config[1] & 4 == 0 { 0 } else { 1 }),
		_ => {
			log_warning!("Request for non-existant attr '{}' on device 0x{:05x}", name, self.addr);
			AttrValue::None
			},
		}
	}
	fn set_attr(&mut self, name: &str, value: ::device_manager::AttrValue) {
		use device_manager::AttrValue;
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
			write_word(self.addr, 1, self.config[1]);
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
	fn bind_io(&mut self, block_id: usize) -> ::device_manager::IOBinding
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
		match parse_bar(self.addr, 4+block_id as u8)
		{
		BAR::None => {
			log_error!("PCI bind_io - Request for BAR{} of {:#x} which isn't populated", block_id, self.addr);
			::device_manager::IOBinding::IO(0,0)
			},
		BAR::IO(b,s) => ::device_manager::IOBinding::IO(b,s),
		BAR::Mem(base, size, _prefetchable) => {
			// TODO: Ensure safety by preventing multiple bindings to a BAR
			// Assume SAFE: Shouldn't be aliased
			let ah = unsafe {::memory::virt::map_mmio(base as ::memory::PAddr, size as usize).unwrap() };
			::device_manager::IOBinding::Memory( ah )
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

fn scan_bus(bus_id: u8) -> Vec<Box<BusDevice+'static>>
{
	log_trace!("PCI scan_bus({})", bus_id);
	let mut ret: Vec<Box<BusDevice>> = Vec::new();
	for devidx in 0 .. MAX_DEV
	{
		match get_device(bus_id, devidx, 0)
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
					if let Some(devinfo) = get_device(bus_id, devidx, fcnidx)
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

fn get_device(bus_id: u8, devidx: u8, function: u8) -> Option<PCIDev>
{
	let addr = get_pci_addr(bus_id, devidx, function);
	let idword = read_word(addr, CONFIG_WORD_IDENT);
	
	if idword & 0xFFFF == 0xFFFF {
		None
	}
	else {
		Some(PCIDev {
			addr: addr,
			vendor: (idword & 0xFFFF) as u16,
			device: (idword >> 16) as u16,
			class: read_word(addr, CONFIG_WORD_CLASS),
			config: [
				idword            , read_word(addr, 1),
				read_word(addr, 2), read_word(addr, 3),
				read_word(addr, 4), read_word(addr, 5),
				read_word(addr, 6), read_word(addr, 7),
				read_word(addr, 8), read_word(addr, 9),
				read_word(addr,10), read_word(addr,11),
				read_word(addr,12), read_word(addr,13),
				read_word(addr,14), read_word(addr,15),
				],
			})
	}
}

fn parse_bar(addr: u16, word: u8) -> BAR
{
	let value = read_word(addr, word);
	if value == 0
	{
		log_debug!("parse_bar: None");
		BAR::None
	}
	else if value & 1 == 0
	{
		write_word(addr, word, 0xFFFFFFFF);
		let one_value = read_word(addr, word);
		let size = !(one_value & 0xFFFF_FFF0) + 1;
		write_word(addr, word, value);
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
			let value2 = read_word(addr, word+1);
			write_word(addr, word+1, !0);
			let size2 = !read_word(addr, word+1) + 1;
			write_word(addr, word+1, value2);
			assert_eq!(size2, 0);
			
			BAR::Mem( (value2 as u64) << 32 | (value as u64 & !0xF), size, pf == 1 )
			},
		3 => BAR::None,	// reserved
		_ => unreachable!()
		}
	}
	else
	{
		// IO BAR
		write_word(addr, word, 0xFFFF);
		let one_value = read_word(addr, word);
		let size = ( !(one_value & 0xFFFC) + 1 ) & 0xFFFF;
		log_debug!("parse_bar: (IO) one_value = {:#x}, size={:#x}, value={:#x}", one_value, size, value);
		write_word(addr, word, value);
		BAR::IO( (value & 0xFFFC) as u16, size as u16 )
	}
}

fn get_pci_addr(bus_id: u8, dev: u8, fcn: u8) -> u16
{
	assert!(dev < MAX_DEV);
	assert!(fcn < MAX_FUNC);
	((bus_id as u16) << 8) | ((dev as u16) << 3) | (fcn as u16)
}

fn read_word(bus_addr: u16, wordidx: u8) -> u32
{
	let addr = ((bus_addr as u32) << 8) | ((wordidx as u32) << 2);
	//log_trace!("read_word(bus_addr={:x},idx={}) addr={:#x}", bus_addr, wordidx, addr);
	::arch::pci::read(addr)
}
fn write_word(bus_addr: u16, wordidx: u8, value: u32)
{
	let addr = ((bus_addr as u32) << 8) | ((wordidx as u32) << 2);
	//log_trace!("read_word(bus_addr={:x},idx={}) addr={:#x}", bus_addr, wordidx, addr);
	::arch::pci::write(addr, value)
}


impl ::core::fmt::Debug for PCIDev
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::result::Result<(),::core::fmt::Error>
	{
		write!(f, "{:#04x} Ven:{:04x} Dev:{:04x} Class {:08x}", self.addr, self.vendor, self.device, self.class)
	}
}

// vim: ft=rust
