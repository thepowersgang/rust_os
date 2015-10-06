//
//
//
use prelude::*;
use super::fdt;

module_define!{FDTDevices, [], init}

struct BusManager;
static S_BUS_MANAGER: BusManager = BusManager;
struct BusDev
{
	node: fdt::Node<'static, 'static>,
	compat: &'static str,
	mmio: Option< (u64, u32) >,
	irq_gsi: Option<u32>,
}

fn init() {
	if let Some(fdt) = super::boot::get_fdt()
	{
		let root_node = fdt.get_nodes(&[]).next().unwrap();
		let (scells,) = decode_value(&root_node, "#size-cells", (1,)).unwrap_or( (0,) );
		let (acells,) = decode_value(&root_node, "#address-cells", (1,)).unwrap_or( (0,) );

		let mut devices: Vec<Box<::device_manager::BusDevice>> = Vec::new();
		for dev in fdt.get_nodes(&[""])
		{
			if let Some(compat) = dev.items().filter_map(|r| match r { ("compatible", fdt::Item::Prop(v)) => Some(v), _ => None }).next()
			{
				let compat = ::core::str::from_utf8(compat).unwrap_or("");
				
				log_debug!("dev '{}' compat = '{}'", dev.name(), compat);
				let mmio = if let Some( (io_base, io_size) ) = decode_value(&dev, "reg", (acells, scells)) {
						log_debug!("- IO {:#x}+{:#x}", io_base, io_size);
						Some( (io_base, io_size as u32) )
					}
					else {
						None
					};

				devices.push( Box::new(BusDev {
					node: dev,
					compat: compat,
					mmio: mmio,
					irq_gsi: None,
					}) );
			}
		}


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
	fn addr(&self) -> u32 {
		self.node.offset() as u32
	}
	fn get_attr(&self, name: &str) -> ::device_manager::AttrValue {
		use device_manager::AttrValue;
		match name
		{
		"compatible" => {
			let v = self.node.get_prop("compatible").map(|v| ::core::str::from_utf8(v).unwrap_or("INVALID")).unwrap_or("");
			AttrValue::String( v )
			},
		_ => AttrValue::None,
		}
	}
	fn set_attr(&mut self, name: &str, value: ::device_manager::AttrValue) {
	}
	fn set_power(&mut self, state: bool) {
	}
	fn bind_io(&mut self, block_id: usize) -> ::device_manager::IOBinding {
		match block_id
		{
		0 => if let Some((base, size)) = self.mmio {
				// TODO: Ensure safety
				// SAFE: Can't easily prove
				let ah = unsafe { ::memory::virt::map_mmio(base as ::memory::PAddr, size as usize).unwrap() };
				::device_manager::IOBinding::Memory( ah )
			}
			else {
				panic!("No MMIO block");
			},
		_ => panic!("Unknown block_id {} for fdt_devices::BusDev::bind_io", block_id),
		}
	}
	fn get_irq(&mut self, idx: usize) -> u32 {
		todo!("get_irq");
	}
}

fn decode_value<T: Tuple<u64>>(dev: &super::fdt::Node, name: &str, cells: T) -> Option<T>
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

