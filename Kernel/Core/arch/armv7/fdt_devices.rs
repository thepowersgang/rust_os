//
//
//
module_define!{FDTDevices, [], init}

fn init() {
	if let Some(fdt) = super::boot::get_fdt()
	{
		let root_node = fdt.get_nodes(&[]).next().unwrap();
		let (scells,) = decode_value(&root_node, "#size-cells", (1,)).unwrap_or( (0,) );
		let (acells,) = decode_value(&root_node, "#address-cells", (1,)).unwrap_or( (0,) );
		for dev in fdt.get_nodes(&[""])
		{
			use super::fdt::Item;
			if let Some(compat) = dev.items().filter_map(|r| match r { ("compatible", Item::Prop(v)) => Some(v), _ => None }).next()
			{
				let compat = ::core::str::from_utf8(compat).unwrap_or("");
				
				log_debug!("dev '{}' compat = '{}'", dev.name(), compat);
				if let Some( (io_base, io_size) ) = decode_value(&dev, "reg", (acells, scells)) {
					log_debug!("- IO {:#x}+{:#x}", io_base, io_size);
				}
				match compat
				{
				"virtio,mmio\0" => {
					// TODO: Create bus device on a virtio bus
					},
				_ => {
					},
				}
			}
		}
	}
}

fn decode_value<T: Tuple<u64>>(dev: &super::fdt::Node, name: &str, cells: T) -> Option<T>
{
	use super::fdt::Item;
	use lib::byteorder::{ReadBytesExt,BigEndian};

	dev.items()
		.filter_map(|(n, v)| if n == name { if let Item::Prop(v) = v { Some(v) } else { None } } else { None } )
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

