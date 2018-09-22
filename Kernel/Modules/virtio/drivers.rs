// 
//
//
//! Device manager "drivers"
use kernel::prelude::*;
use kernel::device_manager;
use devices::NullDevice;


static S_PCI_DRIVER: Pci = Pci;
static S_FDT_MMIO_DRIVER: FdtMmioDriver = FdtMmioDriver;

pub fn register()
{
	device_manager::register_driver(&S_PCI_DRIVER);
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

struct Pci;
impl device_manager::Driver for Pci
{
	fn name(&self) -> &str {
		"virtio-pci"
	}
	fn bus_type(&self) -> &str {
		"pci"
	}
	fn handles(&self, bus_dev: &::kernel::device_manager::BusDevice) -> u32
	{
		let vendor = bus_dev.get_attr("vendor").unwrap_u32();
		let device = bus_dev.get_attr("device").unwrap_u32();
		if vendor == 0x1AF4 && (0x1000 <= device && device <= 0x107F)  {
			2
		}
		else {
			0
		}
	}
	fn bind(&self, bus_dev: &mut ::kernel::device_manager::BusDevice) -> Box<::kernel::device_manager::DriverInstance+'static>
	{
		let irq = bus_dev.get_irq(0);
		// TODO: The IO space may not be in BAR0? Instead referenced in PCI capabilities
		// - The PCI capabilities list includes entries for each region the driver uses, which can sub-slice a BAR
		// - Need to be able to read the capabilities list, AND get a sub-slice of a BAR
		let io = bus_dev.bind_io(0);
		let dev = match bus_dev.get_attr("device").unwrap_u32()
			{
			0x1000 => 1,	// network card
			0x1001 => 2,	// block dev
			0x1002 => 5,	// memory baloon
			0x1003 => 3,	// console
			0x1004 => 8,	// SCSI host
			0x1005 => 4,	// entropy source
			v @ 0x1006 ... 0x1008 => todo!("Unknown PCI ID {:#x}", v),
			0x1009 => 9,	// "9P transport"
			v @ 0x100A ... 0x103F => todo!("Unknown PCI ID {:#x}", v),
			v @ 0x1040 ... 0x107F => v - 0x1040,
			v @ _ => panic!("BUGCHECK: Binding with unexpected PCI device id {:#x}", v),
			};

		::devices::new_boxed::<::interface::Mmio>(dev, io, irq)
	}
}

