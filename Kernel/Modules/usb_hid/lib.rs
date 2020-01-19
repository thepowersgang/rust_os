// 
//
//
//! USB HID (Human Interface Device) driver
#![no_std]
#![feature(linkage)]	// for module_define!
use kernel::prelude::*;

#[macro_use]
extern crate kernel;
extern crate usb_core;

module_define!{usb_hid, [usb_core], init}

fn init()
{
	static USB_DRIVER: Driver = Driver;
	::usb_core::device::register_driver(&USB_DRIVER);
}

struct Driver;
impl ::usb_core::device::Driver for Driver
{
	fn name(&self) -> &str {
		"hid"
	}
	fn matches(&self, _vendor_id: u16, _device_id: u16, class_code: u32) -> ::usb_core::device::MatchLevel {
		use ::usb_core::device::MatchLevel;
		if class_code == 0x03_00_00 {
			MatchLevel::Generic
		}
		else {
			MatchLevel::None
		}
	}
	fn start_device<'a>(&self, ep0: &'a ::usb_core::ControlEndpoint, endpoints: Vec<::usb_core::Endpoint>, descriptors: &[u8]) -> ::usb_core::device::Instance<'a> {
		// 1. Find the HID descriptor in the list
		// 2. Locate the report descriptor (0x22) and get the length
		let mut report_desc_len = 0;
		for d in ::usb_core::hw_decls::IterDescriptors(descriptors)
		{
			// 0x21 = HID Descriptor
			if d[1] == 0x21
			{
				// TODO: Get the header
				let ofs = 6;
				let len = d[0] - ofs;
				if len % 3 != 0 {
					log_error!("Invalid HID descriptor: bad length");
					continue ;
				}
				for sd in d[6..].chunks(3)
				{
					let ty = sd[0];
					let len = sd[1] as u16 | (sd[2] as u16) << 8;
					//log_debug!("USB HID Desc {:02x} len={}", ty, len);
					if ty == 0x22 {
						report_desc_len = len;
					}
				}
			}
		}
		// Hand off to the async code (which isn't borrowing the descriptor list)
		Box::new(Self::start_device_inner(ep0, endpoints, report_desc_len))
	}
}

impl Driver
{
	async fn start_device_inner(ep0: &::usb_core::ControlEndpoint, endpoints: Vec<::usb_core::Endpoint>, report_desc_len: u16)
	{
		// 1. Request that descriptor from the device
		let mut buf = vec![0; report_desc_len as usize];
		let res_len = ep0.read_descriptor_raw(0x1000 | 0x22, 0, &mut buf).await.unwrap();
		assert!(res_len == buf.len(), "Report descriptor size mismatch");

		// 2. Parse the report descriptor, and locate collections of known usage
		// - Use collections to determine what bindings to set up
		let mut state = ReportParseState::default();
		for (id, val) in ReportIterRaw(&buf)
		{
			let op = ReportOp::from_pair(id, val);
			log_debug!("> {:?}", op);
			match op
			{
			ReportOp::Collection(num) => {
				},
			ReportOp::EndCollection => {
				//log_error!("TODO: USB HID start_device - coll={:?}", collection);
				},
			ReportOp::Input(v) => {
				log_debug!("> INPUT {:09b} {:?}", v, state);
				},
			ReportOp::Output(v) => {
				log_debug!("> OUTPUT {:09b} {:?}", v, state);
				},
			ReportOp::Feature(v) => {
				log_debug!("> FEATURE {:09b} {:?}", v, state);
				},
			_ => {},
			}
			state.update(op);
		}

		let mut int_endpoint = None;
		for ep in endpoints
		{
			match ep
			{
			::usb_core::Endpoint::Interrupt(ep) => { int_endpoint = Some(ep); },
			_ => {},
			}
		}
		let int_endpoint = int_endpoint.expect("No interrupt endpoint on a HID device?");

		// 3. Start polling the interrupt endpoint
		// - Use the report descriptor to parse it
		loop
		{
			let d = int_endpoint.wait().await;
			//let mut bs = BitStream::new(&d);

			// Decode input using the report descriptor
			let mut state = ReportParseState::default();
			for (id, val) in ReportIterRaw(&buf)
			{
				let op = ReportOp::from_pair(id, val);
				match op
				{
				ReportOp::Input(_v) =>
					for i in 0 .. state.report_count as usize
					{
						let usage = state.usage.get(i);
						log_debug!("{:x} +{}", usage, state.report_size);
					},
				_ => {},
				}
				state.update(op);
			}

		}
	}
}

#[derive(Debug)]
struct Input
{
	usage: u32,
	bits: usize,
	logical_range: ::core::ops::Range<i32>,
	//physical_range: ::core::ops::Range<i32>,
}

/// Iterate over raw entries in a report descriptor
struct ReportIterRaw<'a>(&'a [u8]);
impl<'a> Iterator for ReportIterRaw<'a>
{
	type Item = (u8, u32);
	fn next(&mut self) -> Option<Self::Item>
	{
		if self.0.len() == 0 {
			None
		}
		else
		{
			fn mk_u32_le(b0: u8, b1: u8, b2: u8, b3: u8) -> u32 {
				b0 as u32 | (b1 as u32) << 8 | (b2 as u32) << 16 | (b3 as u32) << 24
			}
			let op_byte = self.0[0];
			let len;
			let val = match op_byte & 3
				{
				0 => { len = 1; 0 },
				1 => { len = 2; if self.0.len() < len { return None; } mk_u32_le(self.0[1], 0, 0, 0) },
				2 => { len = 3; if self.0.len() < len { return None; } mk_u32_le(self.0[1], self.0[2], 0, 0) },
				3 => { len = 5; if self.0.len() < len { return None; } mk_u32_le(self.0[1], self.0[2], self.0[3], self.0[4]) },
				_ => unreachable!(),
				};
			if op_byte == 0xFC|2 {
			}
			self.0 = &self.0[len..];
			Some( (op_byte, val) )
		}
	}
}

macro_rules! define_ops {
	($name:ident : $($val:expr => $var:ident$(($t:ty))?,)*) => {
		#[derive(Debug)]
		enum $name
		{
			$($var$(($t))?, )*
		}
		
		impl $name
		{
			fn from_pair(id: u8, val: u32) -> ReportOp
			{
				match id & 0xFC
				{
				$(
				$val => $name::$var$((<$t>::from(val)))?,
				)*
				_ => $name::Unk(id, val),
				}
			}
		}
	};
}

#[derive(Debug)]
enum ReportOp
{
	// --- (x0)
	Input(u32),
	Output(u32),
	Collection(u32),
	Feature(u32),
	EndCollection,//(u32),

	// --- Global items (x4)
	UsagePage(u32),
	LogicalMin(i32),
	LogicalMax(i32),
	PhysicalMin(i32),
	PhysicalMax(i32),
	UnitExponent(u32),
	Unit(u32),
	ReportSize(u32),
	ReportId(u32),
	ReportCount(u32),
	Push,
	Pop,

	// --- Local items (x8)
	UsageSingle(u32,bool),
	UsageRangeStart(u32,bool),
	UsageRangeEnd(u32,bool),
	DesignatorSingle(u32),
	DesignatorRangeStart(u32),
	DesignatorRangeEnd(u32),
	_Reserved(u32),
	StringSingle(u32),
	StringRangeStart(u32),
	StringRangeEnd(u32),
	Delimiter,
	LongItem(u32),

	Unk(u8, u32)
}
impl ReportOp
{
	fn from_pair(id: u8, val: u32) -> ReportOp
	{
		fn i32_se(v: u32, sz: u8) -> i32 {
			let sign_bits = match sz
				{
				1 => if v >= 0x80 { !0x7F } else { 0 },
				2 => if v >= 0x8000 { !0x7FFF } else { 0 },
				_ => 0,
				};
			(v | sign_bits) as i32
		}
		match id & 0xFC
		{
		// --- (x0)
		0x80 => ReportOp::Input(val),
		0x90 => ReportOp::Output(val),
		0xA0 => ReportOp::Collection(val),
		0xB0 => ReportOp::Feature(val),
		0xC0 => ReportOp::EndCollection,//(val),

		// --- Global items (x4)
		0x04 => ReportOp::UsagePage(val),
		0x14 => ReportOp::LogicalMin(i32_se(val, id & 3)),
		0x24 => ReportOp::LogicalMax(i32_se(val, id & 3)),
		0x34 => ReportOp::PhysicalMin(i32_se(val, id & 3)),
		0x44 => ReportOp::PhysicalMax(i32_se(val, id & 3)),
		0x54 => ReportOp::UnitExponent(val),
		0x64 => ReportOp::Unit(val),
		0x74 => ReportOp::ReportSize(val),
		0x84 => ReportOp::ReportId(val),
		0x94 => ReportOp::ReportCount(val),
		0xA4 => ReportOp::Push,
		0xB4 => ReportOp::Pop,

		// --- Local items (x8)
		0x08 => ReportOp::UsageSingle(val, id & 0x3 == 3),
		0x18 => ReportOp::UsageRangeStart(val, id & 0x3 == 3),
		0x28 => ReportOp::UsageRangeEnd(val, id & 0x3 == 3),
		0x38 => ReportOp::DesignatorSingle(val),
		0x48 => ReportOp::DesignatorRangeStart(val),
		0x58 => ReportOp::DesignatorRangeEnd(val),
		0x68 => ReportOp::_Reserved(val),
		0x78 => ReportOp::StringSingle(val),
		0x88 => ReportOp::StringRangeStart(val),
		0x98 => ReportOp::StringRangeEnd(val),
		0xA8 => ReportOp::Delimiter,
		0xFC => ReportOp::LongItem(val),
		_ => ReportOp::Unk(id, val),
		}
	}
}

#[derive(Default,Debug)]
struct ReportParseState
{
	//collection: Vec<u32>,
	// Global
	usage_page: u32,
	logical_range: (Option<i32>,Option<i32>),
	physical_range: (Option<i32>,Option<i32>),
	unit_exponent: Option<u32>,
	unit: Option<u32>,

	report_size: u32,
	report_id: Option<u32>,
	report_count: u32,

	// Local, cleared after the next main
	usage: List,
	designator: List,
	string: List,
}
#[derive(Debug)]
enum List
{
	Unset,
	Single(u32),
	ProtoRange(u32),
	Range(u32, u32),
}
impl Default for List {
	fn default() -> Self { List::Unset }
}
impl List
{
	fn set_single(&mut self, v: u32) {
		*self = List::Single(v);
	}
	fn set_start(&mut self, v: u32) {
		*self = List::ProtoRange(v);
	}
	fn set_end(&mut self, v: u32) {
		match *self
		{
		List::ProtoRange(s) => {
			*self = List::Range(s, v);
			},
		_ => {},
		}
	}
	fn get(&self, idx: usize) -> u32 {
		match *self
		{
		List::Unset => 0,
		List::Single(v) => v,
		List::ProtoRange(_v) => 0,
		List::Range(s,e) => {
			if (e - s) as usize <= idx {
				s + idx as u32
			}
			else {
				e
			}
			},
		}
	}
}
impl ReportParseState
{
	fn clear_local(&mut self)
	{
		self.usage = Default::default();
		self.designator = Default::default();
		self.string = Default::default();
	}
	fn update(&mut self, op: ReportOp)
	{
		match op
		{
		ReportOp::Input(_) => { self.clear_local(); },
		ReportOp::Output(_) => { self.clear_local(); },
		ReportOp::Feature(_) => { self.clear_local(); },
		//ReportOp::Collection(v) => self.collection.push(v),
		//ReportOp::EndCollection => { self.collection.pop(); },
		ReportOp::Collection(_) => {},
		ReportOp::EndCollection => {},

		ReportOp::UsagePage(v) => self.usage_page = v << 16,
		ReportOp::LogicalMin(v) => self.logical_range.0 = Some(v),
		ReportOp::LogicalMax(v) => self.logical_range.1 = Some(v),
		ReportOp::PhysicalMin(v) => self.physical_range.0 = Some(v),
		ReportOp::PhysicalMax(v) => self.physical_range.1 = Some(v),
		ReportOp::UnitExponent(v) => self.unit_exponent = Some(v),
		ReportOp::Unit(v) => self.unit = Some(v),
		ReportOp::ReportSize(v) => self.report_size = v,
		ReportOp::ReportId(v) => self.report_id = Some(v),
		ReportOp::ReportCount(v) => self.report_count = v,

		ReportOp::Push => todo!("push"),
		ReportOp::Pop => todo!("pop"),

		ReportOp::UsageSingle(v,is32) => self.usage.set_single( if is32 { 0 } else { self.usage_page } | v),
		ReportOp::UsageRangeStart(v,is32) => self.usage.set_start(if is32 { 0 } else { self.usage_page } | v),
		ReportOp::UsageRangeEnd(v,is32) => self.usage.set_end(if is32 { 0 } else { self.usage_page } | v),

		ReportOp::DesignatorSingle(v) => self.designator.set_single(v),
		ReportOp::DesignatorRangeStart(v) => self.designator.set_start(v),
		ReportOp::DesignatorRangeEnd(v) => self.designator.set_end(v),

		ReportOp::StringSingle(v) => self.string.set_single(v),
		ReportOp::StringRangeStart(v) => self.string.set_start(v),
		ReportOp::StringRangeEnd(v) => self.string.set_end(v),

		ReportOp::Delimiter => todo!("Delimiter"),
		ReportOp::_Reserved(..) => {},
		ReportOp::LongItem(..) => {},
		ReportOp::Unk(..) => {},
		}
	}
}

