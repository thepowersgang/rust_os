// 
//
//
//! Device manager "drivers"
use kernel::prelude::*;
use kernel::device_manager;


struct PciLegacyDriver;	// PCI Legacy ATA (BMDMA, all ports/IRQs legacy)
struct PciNativeDriver;	// PCI Native Mode ATA (all configured via PCI)

#[allow(non_upper_case_globals)]
static s_pci_legacy_driver: PciLegacyDriver = PciLegacyDriver;
#[allow(non_upper_case_globals)]
static s_pci_native_driver: PciNativeDriver = PciNativeDriver;

pub fn register()
{
	device_manager::register_driver(&s_pci_legacy_driver);
	device_manager::register_driver(&s_pci_native_driver);
}

impl device_manager::Driver for PciLegacyDriver
{
	fn name(&self) -> &str {
		"ata-legacy"
	}
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &dyn device_manager::BusDevice) -> u32
	{
		let classcode = bus_dev.get_attr("class").unwrap_u32();
		// [class] [subclass] [IF] [ver]
		// - The 5 masks in two bits representing the channel modes
		if classcode & 0xFFFF0500 == 0x01010000 {
			1	// Handle as weakly as possible (vendor-provided drivers bind higher)
		}
		else {
			0
		}
	}
	fn bind(&self, bus_dev: &mut dyn device_manager::BusDevice) -> Box<dyn device_manager::DriverInstance+'static>
	{
		let bm_io = bus_dev.bind_io(4);
		bus_dev.set_attr("bus_master", device_manager::AttrValue::U32(1));
		Box::new( crate::ControllerRoot::new(0x1F0, 0x3F6, 14,  0x170, 0x376, 15,  bm_io) )
	}
}

impl device_manager::Driver for PciNativeDriver
{
	fn name(&self) -> &str {
		"ata-native"
	}
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &dyn device_manager::BusDevice) -> u32
	{
		let classcode = bus_dev.get_attr("class").unwrap_u32();
		// [class] [subclass] [IF] [ver]
		// IF ~= 0x05 means that both channels are in PCI native mode
		if classcode & 0xFFFF0500 == 0x01010500 {
			1	// Handle as weakly as possible (vendor-provided drivers bind higher)
		}
		else {
			0
		}
	}
	fn bind(&self, bus_dev: &mut dyn device_manager::BusDevice) -> Box<dyn device_manager::DriverInstance+'static>
	{
		let irq = bus_dev.get_irq(0);
		let io_pri = bus_dev.bind_io(0).io_base();
		let st_pri = bus_dev.bind_io(1).io_base() + 2;
		let io_sec = bus_dev.bind_io(2).io_base();
		let st_sec = bus_dev.bind_io(3).io_base() + 2;
		let bm_io = bus_dev.bind_io(4);
		Box::new( crate::ControllerRoot::new(io_pri, st_pri, irq,  io_sec, st_sec, irq,  bm_io) )
	}
}

