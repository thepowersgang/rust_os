//
//
//
use crate::prelude::*;
use crate::lib::fdt;
use crate::memory::PAddr;
use core::convert::TryFrom;
use crate::PAGE_SIZE;

struct BusManager;
static S_BUS_MANAGER: BusManager = BusManager;
struct BusDev
{
	node: fdt::Node<'static, 'static>,
	compat: &'static str,
	irq_gsi: Option<u32>,
	acells: u32,
	scells: u32,
}

/// Architecture-provided interrupt controller handle
pub trait IntController
{
	fn get_gsi(&self, cells: Cells) -> Option<u32>;
}
#[derive(Copy,Clone)]
pub struct Cells<'a>(&'a [u8]);
impl<'a> Cells<'a>
{
	pub fn read_1(&mut self) -> Option<u32> {
		read_v(&mut self.0, 1).map(|v| v as u32)
	}
	pub fn read_2(&mut self) -> Option<u64> {
		read_v(&mut self.0, 2)
	}
	pub fn read_n(&mut self, n: usize) -> Option<u64> {
		read_v(&mut self.0, n as u64)
	}
}
impl<'a> ::core::fmt::Debug for Cells<'a> {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		write!(f, "Cells[")?;
		for mut v in self.0.chunks(4) {
			write!(f, "{:#x},", read_v(&mut v, 1).unwrap())?;
		}
		write!(f, "]")?;
		Ok(())
	}
}
pub struct Reg<'a>
{
	data: &'a [u8],
	acells: u8,
	scells: u8,
}
impl<'a> Reg<'a>
{
	pub fn iter<'s>(&'s self) -> impl Iterator<Item=(u64,u64)>+'s {
		self.data.chunks( ((self.acells + self.scells) as usize) * 4 )
			.map(move |bytes| {
				let mut cells = Cells(bytes);
				//log_debug!("{:x?} {} {}", cells, self.acells, self.scells);
				let a = cells.read_n(self.acells as _).expect("acells");
				let s = cells.read_n(self.scells as _).expect("scells");
				(a, s)
				})
	}
	pub fn iter_paddr<'s>(&'s self) -> impl Iterator<Item=Result<(PAddr,usize),(u64,u64)>>+'s {
		self.iter()
			.map(|(io_base, io_size)|
				match ( PAddr::try_from(io_base), io_size.checked_sub(1).and_then(|v| io_base.checked_add(v)).and_then(|v| PAddr::try_from(v).ok()), )
				{
				(Ok(io_base), Some(_)) => {
					Ok( (io_base, io_size as usize) )
					},
				_ => {
					Err( (io_base, io_size) )
					},
				}
				)
	}
}
#[derive(Copy,Clone)]
pub struct Compat<'a>
{
	data: &'a [u8],
}
impl<'a> Compat<'a>
{
	pub fn matches(&self, n: &str) -> bool {
		self.matches_any(&[n])
	}
	pub fn matches_any(&self, n: &[&str]) -> bool {
		self.iter_strings().any(|v| n.iter().any(|&n| n == v))
	}
	pub fn iter_strings(&self) -> impl Iterator<Item=&str> {
		let s = self.data;
		let s = if s.last() == Some(&0) { &s[..s.len()-1] } else { s };
		s.split(|&v| v == b'\0').map(|v| ::core::str::from_utf8(v).unwrap_or("BADSTR") )
	}
}
impl<'a> ::core::fmt::Debug for Compat<'a> {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		write!(f, "Compat{{")?;
		for s in self.iter_strings() {
			write!(f, "{:?},", s)?;
		}
		write!(f, "}}")?;
		Ok(())
	}
}

pub fn init(make_intc: fn(Compat, Reg)->Option<&'static dyn IntController>)
{
	if let Some(fdt) = super::boot::get_fdt()
	{
		let mut intcs: Vec<(u32, &'static dyn IntController)> = Vec::new();
		// Enumerate interrupt controllers
		// - Look for a device with an `interrupt-controller` field
		walk_tree(fdt, |dev, state| {
			if get_value(&dev, "interrupt-controller").is_some() {
				if let Some((phandle,)) = decode_value(&dev, "phandle", (1,)) {
					let reg = get_value(&dev, "reg").unwrap_or(&[]);
					log_debug!("{:x?} {} {}", reg, state.a_cells, state.s_cells);
					let reg = Reg { data: reg, acells: state.a_cells as u8, scells: state.s_cells as u8 };
					let compat = Compat { data: get_value(&dev, "compatible").unwrap_or(&[]) };
					if let Some(intc) = make_intc(compat, reg) {
						intcs.push( (phandle as u32, intc) )
					}
					else {
						log_error!("Unsupported interrupt controller fdt:{:x} phandle={:#x} {:?}", dev.offset(), phandle, compat);
					}
				}
			}
			});

		let mut devices: Vec<Box<dyn crate::device_manager::BusDevice>> = Vec::new();
		walk_tree(fdt, |dev, state| {
			if let Some(compat) = get_value(&dev, "compatible")
			{
				let reg = get_value(&dev, "reg").unwrap_or(&[]);
				let reg = Reg { data: reg, acells: state.a_cells as u8, scells: state.s_cells as u8 };

				let compat = ::core::str::from_utf8(&compat[..compat.len()-1]).unwrap_or("");
				
				log_debug!("fdt:{:x} = dev '{}' compat = '{}'", dev.offset(), dev.name(), compat);
				let valid = reg.iter_paddr()
					.map(|res|
						match res
						{
						Ok( (io_base, io_size) ) => {
							log_debug!("- IO {:#x}+{:#x}", io_base, io_size);
							true
							},
						Err( (io_base, io_size) ) => {
							log_error!("- IO out of range {:#x}+{:#x}", io_base, io_size);
							false
							},
						})
					.all(|v| v)
					;
				if !valid {
					return ;
				}
				let int_parent = decode_value(&dev, "interrupt-parent", (1,)).map(|(v,)| v as u32).unwrap_or(state.int_parent);
				// If there are interrupts present, decode
				let irq = match get_value(&dev, "interrupts").map(Cells)
					{
					Some(cells) =>
						match intcs.iter().find(|(h,_)| *h == int_parent)
						{
						Some((_,intc)) =>
							match intc.get_gsi(cells)
							{
							Some(v) => {
								log_debug!("- IRQ {}", v);
								Some(v)
								},
							None => {
								log_error!("Cannot map interrupt to a GSI - INTC={:#x}, {:?}", state.int_parent, cells);
								None
								},
							},
						None => {
							log_error!("No registered interrupt controller for {:#x} - {:?}", state.int_parent, cells);
							None
							},
						},
					None => None,
					};

				devices.push( Box::new(BusDev {
					node: dev,
					compat: compat,
					irq_gsi: irq,
					acells: state.a_cells, scells: state.s_cells,
					}) );
			}
		});


		crate::device_manager::register_driver(&PciDriver);
		crate::device_manager::register_bus(&S_BUS_MANAGER, devices);
	}
}

#[derive(Default,Copy,Clone)]
struct State
{
	a_cells: u32,
	s_cells: u32,
	int_parent: u32,
}
impl State
{
	fn update(&mut self, dev: &fdt::Node) {
		if let Some((v,)) = decode_value(&dev, "#size-cells", (1,)) {
			self.s_cells = v as u32;
		}
		if let Some((v,)) = decode_value(&dev, "#address-cells", (1,)) {
			self.a_cells = v as u32;
		}
		if let Some((v,)) = decode_value(&dev, "interrupt-parent", (1,)) {
			self.int_parent = v as u32;
		}
	}
}
fn walk_tree<'a, 'fdt: 'a>(fdt: &'a fdt::FDTRoot<'fdt>, mut cb: impl FnMut(fdt::Node<'a,'fdt>, State))
{
	for root in fdt.get_nodes(&[])
	{
		let mut state = State::default();
		state.update(&root);
		walk_tree_inner(&root, state, &mut cb);
	}
}
fn walk_tree_inner<'a, 'fdt: 'a>(fdt: &fdt::Node<'a, 'fdt>, state: State, cb: &mut impl FnMut(fdt::Node<'a,'fdt>, State))
{
	for dev in fdt.items().filter_map(|(_name,v)| match v { fdt::Item::Node(v) => Some(v), _ => None })
	{
		if let Some(compat) = get_value(&dev, "compatible") {
			if (Compat{data: compat}).matches("simple-bus") {
				let mut state = state;
				if let Some((v,)) = decode_value(&dev, "#size-cells", (1,)) {
					state.s_cells = v as u32;
				}
				if let Some((v,)) = decode_value(&dev, "#address-cells", (1,)) {
					state.a_cells = v as u32;
				}
				if let Some((v,)) = decode_value(&dev, "interrupt-parent", (1,)) {
					state.int_parent = v as u32;
				}
				walk_tree_inner(&dev, state, cb);
				continue ;
			}
		}
		cb(dev, state);
	}
}

impl crate::device_manager::BusManager for BusManager
{
	fn bus_type(&self) -> &str { "fdt" }
	fn get_attr_names(&self) -> &[&str]
	{
		static S_ATTR_NAMES: [&'static str; 1] = ["compatible"];
		&S_ATTR_NAMES
	}
}
impl BusDev
{
	fn get_mmio(&self, block_id: usize) -> Option<(PAddr, usize)>
	{
		if let Some(reg) = get_value(&self.node, "reg")
		{
			let reg = Reg { data: reg, acells: self.acells as u8, scells: self.scells as u8 };
			let rv = reg.iter_paddr().skip(block_id).next();
			rv.map(|v| v.expect("Should have been checked earlier"))
		}
		else
		{
			None
		}
	}
}
impl crate::device_manager::BusDevice for BusDev
{
	fn type_id(&self) -> ::core::any::TypeId {
		::core::any::TypeId::of::<Self>()
	}
	fn addr(&self) -> u32 {
		self.node.offset() as u32
	}
	fn get_attr_idx(&self, name: &str, idx: usize) -> crate::device_manager::AttrValue {
		use crate::device_manager::AttrValue;
		match name
		{
		"compatible" if idx == 0 => AttrValue::String(self.compat),
		_ => AttrValue::None,
		}
	}
	fn set_attr_idx(&mut self, _name: &str, _idx: usize, _value: crate::device_manager::AttrValue) {
	}
	fn set_power(&mut self, _state: bool) {
	}
	fn bind_io_slice(&mut self, block_id: usize, slice: Option<(usize,usize)>) -> crate::device_manager::IOBinding {
		if let Some((mut base, mut size)) = self.get_mmio(block_id)
		{
			if let Some( (ofs, subsize) ) = slice {
				assert!(ofs < size, "");
				assert!(ofs + subsize <= size);

				base += ofs as crate::memory::PAddr;
				size = subsize;
			}
			// TODO: Ensure safety
			// SAFE: Can't easily prove
			let ah = unsafe { crate::memory::virt::map_mmio(base, size).unwrap() };
			crate::device_manager::IOBinding::Memory( ah )
		}
		else
		{
			panic!("Unknown block_id {} for fdt_devices::BusDev::bind_io_slice", block_id);
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

fn read_v(bytes: &mut &[u8], size: u64) -> Option<u64>
{
	use crate::lib::byteorder::{ReadBytesExt,BigEndian};

	Some(match size
	{
	1 => bytes.read_u32::<BigEndian>().ok()? as u64,
	2 => bytes.read_u64::<BigEndian>().ok()?,
	_ => return None,
	})
}
fn get_value<'a>(dev: &fdt::Node<'_, 'a>, name: &str) -> Option<&'a [u8]> {
	dev.items()
		.filter_map(|(n, v)| if n == name { if let fdt::Item::Prop(v) = v { Some(v) } else { None } } else { None } )
		.next()
}
fn decode_value<T: Tuple<u64>>(dev: &fdt::Node, name: &str, cells: T) -> Option<T>
{
	get_value(dev, name)
		.map(|mut bytes|
			cells.map(|v| read_v(&mut bytes, v).unwrap_or(0))
			)
}
fn decode_values<U, T: Tuple<u64> + Copy>(dev: &fdt::Node, name: &str, cells: T, mut cb: impl FnMut(T)->Option<U>) -> Option<U>
{
	if let Some(mut bytes) = get_value(dev, name)
	{
		let mut err = false;
		while !err && bytes.len() > 0
		{
			if let Some(v) = cb(cells.map(|v| if let Some(v) = read_v(&mut bytes, v) { v } else { err = true; 0 })) {
				return Some(v);
			}
		}
	}
	None
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


struct PciDriver;
impl crate::device_manager::Driver for PciDriver
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
	fn bind(&self, bus_dev: &mut dyn crate::device_manager::BusDevice) -> Box<dyn crate::device_manager::DriverInstance>
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
			mmio: (crate::memory::PAddr, usize),
			lock: crate::sync::Mutex<Inner>,
		}
		impl Interface
		{
			fn locked<T>(&self, bus_addr: u16, word_idx: u8, cb: impl FnOnce(*mut u32)->T) -> T
			{
				let addr = ((bus_addr as u32) << 8) | ((word_idx as u32) << 2);
				assert!(addr < self.mmio.1 as u32);
				let mut lh = self.lock.lock();
				let base = addr & !(PAGE_SIZE - 1) as u32;
				let ofs  = addr &  (PAGE_SIZE - 1) as u32;
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
		let mmio = d.get_mmio(0).expect("No MMIO for PCI?");
		let int = Aref::new(Interface {
			mmio: mmio,
			lock: crate::sync::Mutex::new(Inner {
				base: 0,
				// SAFE: Owned MMIO memory from device
				mapping: unsafe { crate::memory::virt::map_hw_rw(mmio.0 as PAddr, 1, "fdt_pci").expect("Unable to map PCI") },
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
