// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/pci.rs
// - PCI Bus Handling
use _common::*;

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

module_define!(PCI, [DeviceManager], init)

fn init()
{
	// 1. Enumerate PCI bus(es)
	let devs = scan_bus(0);
	log_debug!("devs = {}", devs);
	// - All drivers that have PCI bindings should be waiting on this to load
}

fn scan_bus(bus_id: u8) -> Vec<PCIDev>
{
	log_trace!("PCI scan_bus({})", bus_id);
	let mut ret = Vec::new();
	for devidx in range(0, MAX_DEV)
	{
		match get_device(bus_id, devidx, 0)
		{
		Some(devinfo) => {
			let is_multifunc = (devinfo.config[3] & 0x0080_0000) != 0;
			// Increase device count
			ret.push(devinfo);
			// Handle multi-function devices (iterate from 1 onwards)
			if is_multifunc
			{
				for fcnidx in range(1, MAX_FUNC)
				{
					if let Some(devinfo) = get_device(bus_id, devidx, fcnidx)
					{
						ret.push(devinfo);
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
