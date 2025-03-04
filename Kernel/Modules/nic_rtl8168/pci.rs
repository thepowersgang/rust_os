//! PCI bus binding/driver
use ::kernel::device_manager;

pub static DRIVER: PciDriver = PciDriver;


pub struct PciDriver;
impl device_manager::Driver for PciDriver {
	fn name(&self) -> &str {
		"rtl8168-pci"
	}
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &dyn device_manager::BusDevice) -> u32 {
		let vendor = bus_dev.get_attr("vendor").unwrap_u32();
		let device = bus_dev.get_attr("device").unwrap_u32();
		if vendor == 0x10ec && device == 0x8129 {
			2
		}
		else {
			0
		}
	}
	fn bind(&self, bus_dev: &mut dyn device_manager::BusDevice) -> device_manager::DriverBindResult {
		let irq = bus_dev.get_irq(0);
		let base = bus_dev.bind_io(0);

		Ok(device_manager::DriverInstancePtr::new( super::BusDev::new(irq, base)? ))
	}
}
