use kernel::prelude::*;

pub struct PciDriver;

impl ::kernel::device_manager::Driver for PciDriver {
	fn name(&self) -> &str {
		"ohci-pci"
	}
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &::kernel::device_manager::BusDevice) -> u32
	{
		let class = bus_dev.get_attr("class").unwrap_u32();
		if class & 0xFF_FF_FF_00 == 0x0C0310_00 { 
			1
		}
		else {
			0
		}
	}
	fn bind(&self, bus_dev: &mut ::kernel::device_manager::BusDevice) -> Box<::kernel::device_manager::DriverInstance+'static>
	{
		let irq = bus_dev.get_irq(0);
		let base = bus_dev.bind_io(0);

		::BusDev::new_boxed(irq, base).expect("ohci")
	}
}

