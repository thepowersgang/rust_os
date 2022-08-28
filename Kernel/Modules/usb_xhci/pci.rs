//! PCI device bindings
use kernel::device_manager;

pub struct PciDriver;

impl device_manager::Driver for PciDriver {
	fn name(&self) -> &str {
		"xhci-pci"
	}
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &dyn device_manager::BusDevice) -> u32
	{
		let class = bus_dev.get_attr("class").unwrap_u32();
		if class & 0xFF_FF_FF_00 == 0x0C0330_00 { 
			1
		}
		else {
			0
		}
	}
	fn bind(&self, bus_dev: &mut dyn device_manager::BusDevice) -> device_manager::DriverBindResult
	{
		let irq = bus_dev.get_irq(0);
		let base = bus_dev.bind_io(0);

		Ok( device_manager::DriverInstancePtr::new(BusDev::new(irq, base)?) )
	}
}


struct BusDev(::kernel::lib::mem::aref::Aref<super::HostInner>);
impl BusDev
{
	fn new(irq: u32, io: ::kernel::device_manager::IOBinding) -> Result<Self, ::kernel::device_manager::DriverBindError> {
		Ok(BusDev(super::HostInner::new_aref(irq, io)?))
	}
}
impl ::kernel::device_manager::DriverInstance for BusDev
{
}
