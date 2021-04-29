//
//
//
use prelude::*;
use crate::lib::fdt;
use core::convert::TryFrom;

module_define!{FDTDevices, [], init}

struct BusManager;
static S_BUS_MANAGER: BusManager = BusManager;
struct BusDev
{
	node: fdt::Node<'static, 'static>,
	compat: &'static str,
	mmio: Option< (crate::memory::PAddr, u32) >,
	irq_gsi: Option<u32>,
}

fn init()
{
	if let Some(fdt) = super::boot::get_fdt()
	{
		let root_node = fdt.get_nodes(&[]).next().unwrap();
		let (scells,) = decode_value(&root_node, "#size-cells", (1,)).unwrap_or( (0,) );
		let (acells,) = decode_value(&root_node, "#address-cells", (1,)).unwrap_or( (0,) );

		let mut devices: Vec<Box<dyn crate::device_manager::BusDevice>> = Vec::new();
		for dev in Iterator::chain( fdt.get_nodes(&[""]), fdt.get_nodes(&["", "soc"]) )
		{
			if let Some(compat) = dev.items().filter_map(|r| match r { ("compatible", fdt::Item::Prop(v)) => Some(v), _ => None }).next()
			{
				let compat = ::core::str::from_utf8(&compat[..compat.len()-1]).unwrap_or("");
				
				log_debug!("fdt:{:x} = dev '{}' compat = '{}'", dev.offset(), dev.name(), compat);
				let mmio = if let Some( (io_base, io_size) ) = decode_value(&dev, "reg", (acells, scells)) {
						match crate::memory::PAddr::try_from(io_base)
						{
						Err(_) => {
							log_error!("- IO out of range {:#x}+{:#x}", io_base, io_size);
							continue
							},
						Ok(io_base) => match crate::memory::PAddr::try_from(io_base as u64 + io_size as u64 - 1)
							{
							Err(_) => {
								log_error!("- IO out of range {:#x}+{:#x}", io_base, io_size);
								continue
								},
							Ok(_) => {
								log_debug!("- IO {:#x}+{:#x}", io_base, io_size);
								Some( (io_base, io_size as u32) )
								}
							},
						}
					}
					else {
						None
					};
				let irq = decode_value(&dev, "interrupts", (1,)).map(|v| v.0 as u32);

				devices.push( Box::new(BusDev {
					node: dev,
					compat: compat,
					mmio: mmio,
					irq_gsi: irq,
					}) );
			}
		}


		::device_manager::register_driver(&PciDriver);
		//::device_manager::register_driver(&SubBus);
		::device_manager::register_bus(&S_BUS_MANAGER, devices);
	}
}

impl ::device_manager::BusManager for BusManager
{
	fn bus_type(&self) -> &str { "fdt" }
	fn get_attr_names(&self) -> &[&str]
	{
		static S_ATTR_NAMES: [&'static str; 1] = ["compatible"];
		&S_ATTR_NAMES
	}
}
impl ::device_manager::BusDevice for BusDev
{
	fn type_id(&self) -> ::core::any::TypeId {
		::core::any::TypeId::of::<Self>()
	}
	fn addr(&self) -> u32 {
		self.node.offset() as u32
	}
	fn get_attr_idx(&self, name: &str, idx: usize) -> ::device_manager::AttrValue {
		use device_manager::AttrValue;
		match name
		{
		"compatible" if idx == 0 => AttrValue::String(self.compat),
		_ => AttrValue::None,
		}
	}
	fn set_attr_idx(&mut self, _name: &str, _idx: usize, _value: ::device_manager::AttrValue) {
	}
	fn set_power(&mut self, _state: bool) {
	}
	fn bind_io_slice(&mut self, block_id: usize, slice: Option<(usize,usize)>) -> crate::device_manager::IOBinding {
		match block_id
		{
		0 => if let Some((mut base, mut size)) = self.mmio {
				if let Some( (ofs, subsize) ) = slice {
					assert!(ofs < size as usize, "");
					assert!(ofs + subsize <= size as usize);

					base += ofs as crate::memory::PAddr;
					size = subsize as u32;
				}
				// TODO: Ensure safety
				// SAFE: Can't easily prove
				let ah = unsafe { ::memory::virt::map_mmio(base, size as usize).unwrap() };
				::device_manager::IOBinding::Memory( ah )
			}
			else {
				panic!("No MMIO block");
			},
		_ => panic!("Unknown block_id {} for fdt_devices::BusDev::bind_io_slice", block_id),
		}
	}
	fn get_irq(&mut self, idx: usize) -> u32 {
		if idx != 0 {
			panic!("Invalid IRQ index {}", idx);
		}
		//self.irq_gsi.expect("FDT Devices - No IRQ")
		self.irq_gsi.unwrap_or(0)
	}
}

fn decode_value<T: Tuple<u64>>(dev: &fdt::Node, name: &str, cells: T) -> Option<T>
{
	use lib::byteorder::{ReadBytesExt,BigEndian};

	dev.items()
		.filter_map(|(n, v)| if n == name { if let fdt::Item::Prop(v) = v { Some(v) } else { None } } else { None } )
		.next()
		.map(|mut bytes|
			cells.map(
				|v| match v
					{
					1 => bytes.read_u32::<BigEndian>().unwrap_or(0) as u64,
					2 => bytes.read_u64::<BigEndian>().unwrap_or(0),
					_ => 0,
					}
				)
			)
}


trait Tuple<T> {
	fn map<F>(self, f: F) -> Self where F: FnMut(T)->T;
}
impl<T> Tuple<T> for (T,) {
	fn map<F>(self, mut f: F) -> Self where F: FnMut(T)->T {
		(f(self.0),)
	}
}
impl<T> Tuple<T> for (T,T,) {
	fn map<F>(self, mut f: F) -> Self where F: FnMut(T)->T {
		(f(self.0),f(self.1),)
	}
}

struct SubBus;
impl crate::device_manager::Driver for SubBus
{
	fn name(&self) -> &str {
		"fdt:simple-bus"
	}
	fn bus_type(&self) -> &str {
		"fdt"
	}
	fn handles(&self, bus_dev: &dyn crate::device_manager::BusDevice) -> u32
	{
		let d = bus_dev.downcast_ref::<BusDev>().expect("Not a FDT device?");
		log_trace!("SubBus - {}", d.compat);
		match d.compat
		{
		"simple-bus" => 1,
		_ => 0,
		}
	}
	fn bind(&self, bus_dev: &mut dyn crate::device_manager::BusDevice) -> Box<dyn (::device_manager::DriverInstance)>
	{
		assert!(self.handles(&*bus_dev) > 0);
		//let d = bus_dev.downcast_ref::<BusDev>().expect("Not a FDT device?");
		
		todo!("SubBus::bind");
	}
}

struct PciDriver;
impl ::device_manager::Driver for PciDriver
{
	fn name(&self) -> &str {
		"fdt:pci"
	}
	fn bus_type(&self) -> &str {
		"fdt"
	}
	fn handles(&self, bus_dev: &dyn crate::device_manager::BusDevice) -> u32
	{
		let d = bus_dev.downcast_ref::<BusDev>().expect("Not a FDT device?");
		match d.compat
		{
		"pci-host-ecam-generic" => 1,
		_ => 0,
		}
	}
	fn bind(&self, bus_dev: &mut dyn crate::device_manager::BusDevice) -> Box<dyn (::device_manager::DriverInstance)>
	{
		assert!(self.handles(&*bus_dev) > 0);
		let d = bus_dev.downcast_ref::<BusDev>().expect("Not a FDT device?");
		
		use crate::lib::mem::aref::Aref;
		use crate::hw::bus_pci;
		use crate::memory::PAddr;
		use ::core::ptr::{read_volatile,write_volatile};
		struct Inner
		{
			base: u32,
			mapping: crate::memory::virt::AllocHandle,
		}
		struct Interface
		{
			mmio: (crate::memory::PAddr, u32),
			lock: crate::sync::Mutex<Inner>,
		}
		impl Interface
		{
			fn locked<T>(&self, bus_addr: u16, word_idx: u8, cb: impl FnOnce(*mut u32)->T) -> T
			{
				let addr = ((bus_addr as u32) << 8) | ((word_idx as u32) << 2);
				assert!(addr < self.mmio.1);
				let mut lh = self.lock.lock();
				let base = addr & !(::PAGE_SIZE - 1) as u32;
				let ofs  = addr &  (::PAGE_SIZE - 1) as u32;
				if lh.base != base {
					// SAFE: Owned MMIO memory from device
					lh.mapping = unsafe { crate::memory::virt::map_hw_rw(self.mmio.0 + base as PAddr, 1, "fdt_pci").expect("Unable to map PCI") };
					lh.base = base;
				}
				cb( lh.mapping.as_ref::<u32>(ofs as usize) as *const _ as *mut _ )
			}
		}
		impl bus_pci::PciInterface for Interface
		{
			fn read_word(&self, bus_addr: u16, word_idx: u8) -> u32 {
				// SAFE: Reading the PCI config space is safe
				self.locked(bus_addr, word_idx, |ptr| unsafe { read_volatile(ptr) })
			}
			unsafe fn write_word(&self, bus_addr: u16, word_idx: u8, val: u32) {
				self.locked(bus_addr, word_idx, |ptr| write_volatile(ptr, val))
			}
			unsafe fn get_mask(&self, bus_addr: u16, word_idx: u8, in_mask: u32) -> (u32, u32) {
				self.locked(bus_addr, word_idx, |ptr| {
					let old_value = read_volatile(ptr);
					write_volatile(ptr, in_mask);
					let new_value = read_volatile(ptr);
					write_volatile(ptr, old_value);
					(old_value, new_value)
					})
			}
		}
		let int = Aref::new(Interface {
			mmio: d.mmio.unwrap(),
			lock: crate::sync::Mutex::new(Inner {
				base: 0,
				// SAFE: Owned MMIO memory from device
				mapping: unsafe { crate::memory::virt::map_hw_rw(d.mmio.unwrap().0 as PAddr, 1, "fdt_pci").expect("Unable to map PCI") },
				}),
			});
		log_debug!("FDT PCI: {:x?}", int.mmio);
		// Enumerate the bus
		bus_pci::register_bus(int.borrow());
		struct Instance
		{
			_int: Aref<Interface>,
		}
		impl crate::device_manager::DriverInstance for Instance
		{
		}
		Box::new(Instance { _int: int, })
	}
}
