// "Tifflin" Kernel - AHCI (SATA) Driver
// - By John Hodge (thePowersGang)
//
// Modules/storage_ahci/bus_bindings.rs
//! Bus drivers (e.g. PCI)
use kernel::prelude::*;
use kernel::device_manager;

pub static S_PCI_DRIVER: PciDriver = PciDriver;

/// Standard PCI bus binding (Class 1, Subclass 6, IF 1)
pub struct PciDriver;

impl device_manager::Driver for PciDriver
{
	fn name(&self) -> &str {
		"ahci-pci"
	}
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &::kernel::device_manager::BusDevice) -> u32
	{
		let classcode = bus_dev.get_attr("class").unwrap_u32();
		// [class] [subclass] [IF] [ver]
		if classcode & 0xFFFFFF00 == 0x01060100 {
			1	// Handle as weakly as possible (vendor-provided drivers bind higher)
		}
		else {
			0
		}
	}
	fn bind(&self, bus_dev: &mut ::kernel::device_manager::BusDevice) -> Box<::kernel::device_manager::DriverInstance+'static>
	{
		let irq = bus_dev.get_irq(0);
		let base = bus_dev.bind_io(5);

		::controller::Controller::new(irq, base).unwrap()
	}
}
