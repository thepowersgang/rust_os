// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/pci.rs
// - PCI Bus Handling
use _common::*;

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
	config: [u32,..16]
}

struct PCIBusManager;
struct PCIChildBusDriver;

#[allow(non_upper_case_globals)]
static s_pci_bus_manager: PCIBusManager = PCIBusManager;
#[allow(non_upper_case_globals)]
static s_pci_child_bus_driver: PCIChildBusDriver = PCIChildBusDriver;
static S_ATTR_NAMES: [&'static str, ..3] = ["vendor", "device", "class"];

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
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &::device_manager::BusDevice) -> uint
	{
		let addr = bus_dev.addr() as u16;
		let bridge_type = (read_word(addr, 3) >> 16) & 0x7F;
		// 0x00 == Normal device, 0x01 = PCI-PCI Bridge
		// -> There should only be one PCI bridge handler, but bind low just in case
		if bridge_type == 0x01 { 1 } else {0 }
	}
	fn bind(&self, bus_dev: &::device_manager::BusDevice) -> Box<::device_manager::DriverInstance+'static>
	{
		let addr = bus_dev.addr() as u16;
		let bridge_type = (read_word(addr, 3) >> 16) & 0x7F;
		assert!(bridge_type == 0x01);
		// Get sub-bus number
		let sec_bus_id = (read_word(addr, 6) >> 8) & 0xFF;
		log_debug!("PCI Bridge Bind: sec_bus_id = {:#02x}", sec_bus_id);
		panic!("TODO");
	}
}

impl ::device_manager::BusDevice for PCIDev
{
	fn addr(&self) -> u32 {
		self.addr as u32
	}
	fn get_attr(&self, name: &str) -> u32 {
		match name
		{
		_ => 0,
		}
	}
	fn set_power(&mut self, state: bool)
	{
		// Nope
		panic!("TODO: Set power state of PCI devices (state={})", state);
	}
	fn bind_io(&mut self, block_id: uint) -> ::device_manager::IOBinding
	{
		panic!("TODO: PCIDev::bind_io(block_id={})", block_id);
	}
}

fn scan_bus(bus_id: u8) -> Vec<Box<BusDevice+'static>>
{
	log_trace!("PCI scan_bus({})", bus_id);
	let mut ret = Vec::new();
	for devidx in range(0, MAX_DEV)
	{
		match get_device(bus_id, devidx, 0)
		{
		Some(devinfo) => {
			let is_multifunc = (devinfo.config[3] & 0x0080_0000) != 0;
			log_debug!("{}", devinfo);
			// Increase device count
			ret.push(box devinfo as Box<BusDevice>);
			// Handle multi-function devices (iterate from 1 onwards)
			if is_multifunc
			{
				for fcnidx in range(1, MAX_FUNC)
				{
					if let Some(devinfo) = get_device(bus_id, devidx, fcnidx)
					{
						log_debug!("{}", devinfo);
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
		log_debug!("Device {:#x} = Dev/Ven {:08x}", addr, idword);
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

fn get_pci_addr(bus_id: u8, dev: u8, fcn: u8) -> u16
{
	assert!(dev < MAX_DEV);
	assert!(fcn < MAX_FUNC);
	(bus_id as u16 << 8) | (dev as u16 << 3) | fcn as u16
}

fn read_word(bus_addr: u16, wordidx: u8) -> u32
{
	let addr = (bus_addr as u32 << 8) | (wordidx as u32 << 2);
	//log_trace!("read_word(bus_addr={:x},idx={}) addr={:#x}", bus_addr, wordidx, addr);
	::arch::pci::read(addr)
}


impl ::core::fmt::Show for PCIDev
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::result::Result<(),::core::fmt::Error>
	{
		write!(f, "{:#04x} V{:04x} D{:04x} {:06x}", self.addr, self.vendor, self.device, self.class)
	}
}

// vim: ft=rust
