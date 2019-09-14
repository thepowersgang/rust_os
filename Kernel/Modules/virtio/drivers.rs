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
	fn handles(&self, bus_dev: &dyn device_manager::BusDevice) -> u32
	{
		if bus_dev.get_attr("compatible").unwrap_str() == "virtio,mmio\0" {
			1
		}
		else {
			0
		}
	}
	fn bind(&self, bus_dev: &mut dyn device_manager::BusDevice) -> Box<dyn device_manager::DriverInstance+'static>
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

		::devices::new_boxed(dev, ::interface::Mmio::new(io, bus_dev.get_irq(0)))
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
	fn handles(&self, bus_dev: &dyn device_manager::BusDevice) -> u32
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
	fn bind(&self, bus_dev: &mut dyn device_manager::BusDevice) -> Box<dyn device_manager::DriverInstance+'static>
	{
		let irq = bus_dev.get_irq(0);
		// TODO: The IO space may not be in BAR0? Instead referenced in PCI capabilities
		// - The PCI capabilities list includes entries for each region the driver uses, which can sub-slice a BAR
		// - Need to be able to read the capabilities list, AND get a sub-slice of a BAR
		let dev = match bus_dev.get_attr("device").unwrap_u32()
			{
			0x1000 => 1,	// network card
			0x1001 => 2,	// block dev
			0x1002 => 5,	// memory baloon
			0x1003 => 3,	// console
			0x1004 => 8,	// SCSI host
			0x1005 => 4,	// entropy source
			v @ 0x1006 ..= 0x1008 => todo!("Unknown PCI ID {:#x}", v),
			0x1009 => 9,	// "9P transport"
			v @ 0x100A ..= 0x103F => todo!("Unknown PCI ID {:#x}", v),
			v @ 0x1040 ..= 0x107F => v - 0x1040,
			v @ _ => panic!("BUGCHECK: Binding with unexpected PCI device id {:#x}", v),
			};

		let mut common_bar = None;
		let mut device_cfg_bar = None;
		let mut notify_bar = None;
		for cap in pci_helpers::CapabilityIter::new(&*bus_dev)
		{
			match cap.id
			{
			9 => {
				let bar = cap.read_32(1) as usize;
				let ofs = cap.read_32(2) as usize;
				let len = cap.read_32(3) as usize;
				match cap.byte0
				{
				1 => {
					log_debug!("Common: BAR{} {:#x}+{:#x}", bar, ofs, len);
					let io = (bar, ofs, len);
					if common_bar.is_none() {
						common_bar = Some(io);
					}
					},
				2 => {
					log_debug!("Notify: BAR{} {:#x}+{:#x}", bar, ofs, len);
					let io = (bar, ofs, len);
					let mult = cap.read_32(4);

					if notify_bar.is_none() {
						notify_bar = Some( (io, mult,) );
					}
					},
				3 => log_debug!("Isr: BAR{} {:#x}+{:#x}", bar, ofs, len),
				4 => {
					log_debug!("Device Config: BAR{} {:#x}+{:#x}", bar, ofs, len);
					let io = (bar, ofs, len);
					if device_cfg_bar.is_none() {
						device_cfg_bar = Some(io);
					}
					},
				5 => log_debug!("PCI CFG: BAR{} {:#x}+{:#x}", bar, ofs, len),
				_ => {},
				}
				},
			_ => {},
			}
		}

		match (common_bar, device_cfg_bar, notify_bar)
		{
		( Some(common), Some(dev_cfg), Some( (notify, notify_mult) ) ) => {
			let io = ::interface::PciRegions {
				common: bus_dev.bind_io_slice( common.0, Some((common.1, common.2)) ),
				notify: bus_dev.bind_io_slice( notify.0, Some((notify.1, notify.2)) ),
				notify_off_mult: notify_mult,
				dev_cfg: bus_dev.bind_io_slice( dev_cfg.0, Some((dev_cfg.1, dev_cfg.2)) ),
				};
			::devices::new_boxed(dev, ::interface::Pci::new(io, irq))
			},
		(common_bar, device_cfg_bar, notify_bar,) => {
			log_error!("VirtIO PCI device doesn't have a full set of capabilities - common={:?} dev_cfg={:?} notify={:?}", common_bar, device_cfg_bar, notify_bar);
			return Box::new( NullDevice );
			},
		}
	}
}

mod pci_helpers
{
	use kernel::device_manager;
	pub struct Capability<'a> {
		dev: &'a dyn device_manager::BusDevice,
		pub id: u8,
		ofs: u8,
		len: u8,
		pub byte0: u8,
	}
	impl<'a> Capability<'a>
	{
		pub fn read_32(&self, idx: usize) -> u32 {
			assert!(idx < self.len as usize / 4);
			self.dev.get_attr_idx("raw_config", self.ofs as usize + idx*4).unwrap_u32()
		}
	}

	pub struct CapabilityIter<'a>
	{
		dev: &'a dyn kernel::device_manager::BusDevice,
		cap_ptr: u8,
	}
	impl<'a> CapabilityIter<'a>
	{
		pub fn new(dev: &'a dyn device_manager::BusDevice) -> Self
		{
			// TODO: Assert that it's PCI
			CapabilityIter {
				dev: dev,
				cap_ptr: if (dev.get_attr_idx("raw_config", 0x4).unwrap_u32() >> 16) & 0x10 != 0 {
						(dev.get_attr_idx("raw_config", 0x34).unwrap_u32() & 0xFC) as u8
					}
					else {
						0
					},
				}
		}
	}
	impl<'a> Iterator for CapabilityIter<'a>
	{
		type Item = Capability<'a>;
		fn next(&mut self) -> Option<Capability<'a>>
		{
			if self.cap_ptr == 0 {
				None
			}
			else {
				let cap_hdr = self.dev.get_attr_idx("raw_config", self.cap_ptr as usize).unwrap_u32();
				let (id, next, len, byte0) = (cap_hdr as u8, (cap_hdr >> 8) as u8, (cap_hdr >> 16) as u8, (cap_hdr >> 24) as u8);
				let rv = Capability {
					dev: self.dev,
					id: id,
					ofs: self.cap_ptr,
					len: len,
					byte0: byte0,
					};
				self.cap_ptr = next;
				Some(rv)
			}
		}
	}
}
