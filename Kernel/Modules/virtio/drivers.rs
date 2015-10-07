// 
//
//
//! Device manager "drivers"
use kernel::prelude::*;
use kernel::device_manager;
use devices::NullDevice;


static S_FDT_MMIO_DRIVER: FdtMmioDriver = FdtMmioDriver;

pub fn register()
{
	device_manager::register_driver(&S_FDT_MMIO_DRIVER);
}


struct FdtMmioDriver;
impl device_manager::Driver for FdtMmioDriver
{
	fn name(&self) -> &str {
		"virtio-fdt-mmio"
	}
	fn bus_type(&self) -> &str {
		"fdt"
	}
	fn handles(&self, bus_dev: &device_manager::BusDevice) -> u32
	{
		if bus_dev.get_attr("compatible").unwrap_str() == "virtio,mmio\0" {
			1
		}
		else {
			0
		}
	}
	fn bind(&self, bus_dev: &mut device_manager::BusDevice) -> Box<device_manager::DriverInstance+'static>
	{
		let io = bus_dev.bind_io(0);
		log_debug!("io = {:?}", io);
		// SAFE: No-sideeffect IO read
		let (magic, ver, ven, dev) = unsafe { (io.read_32(0), io.read_32(4),  io.read_32(12), io.read_32(8)) };
		log_debug!("Magic = {:#x}, version={:#x},  Vendor:Device = {:#x}:{:#x}", magic, ver, ven, dev);

		const MAGIC: u32 = 0x74726976;	// "virt"
		if magic != MAGIC {
			log_error!("VirtIO device invalid magic {:#x} != exp {:#x}", magic, MAGIC);
			return Box::new( NullDevice );
		}

		::devices::new_boxed::<::interface::Mmio>(dev, io, bus_dev.get_irq(0))
	}
}


