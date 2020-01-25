// "Tifflin" Kernel - USB HID driver
// - By John Hodge (Mutabah / thePowersGang)
//
// Modules/usb_hid/report_parser.rs
//! Parser for USB HID report descriptors

/// Iterate over raw entries in a report descriptor
pub struct IterRaw<'a>(pub &'a [u8]);
impl<'a> Iterator for IterRaw<'a>
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
				todo!("Handle long entries");
			}
			self.0 = &self.0[len..];
			Some( (op_byte, val) )
		}
	}
}

/// A parsed operator in a report descriptor
#[derive(Debug)]
pub enum Op
{
	// --- (x0)
	Input(InputFlags),
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
impl Op
{
	/// Parse a pair of ID and value into an `Op`
	pub fn from_pair(id: u8, val: u32) -> Op
	{
		/// Get a sign-extended i32
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
		0x80 => Op::Input( InputFlags(val) ),
		0x90 => Op::Output(val),
		0xA0 => Op::Collection(val),
		0xB0 => Op::Feature(val),
		0xC0 => Op::EndCollection,//(val),

		// --- Global items (x4)
		0x04 => Op::UsagePage(val),
		0x14 => Op::LogicalMin(i32_se(val, id & 3)),
		0x24 => Op::LogicalMax(i32_se(val, id & 3)),
		0x34 => Op::PhysicalMin(i32_se(val, id & 3)),
		0x44 => Op::PhysicalMax(i32_se(val, id & 3)),
		0x54 => Op::UnitExponent(val),
		0x64 => Op::Unit(val),
		0x74 => Op::ReportSize(val),
		0x84 => Op::ReportId(val),
		0x94 => Op::ReportCount(val),
		0xA4 => Op::Push,
		0xB4 => Op::Pop,

		// --- Local items (x8)
		0x08 => Op::UsageSingle(val, id & 0x3 == 3),
		0x18 => Op::UsageRangeStart(val, id & 0x3 == 3),
		0x28 => Op::UsageRangeEnd(val, id & 0x3 == 3),
		0x38 => Op::DesignatorSingle(val),
		0x48 => Op::DesignatorRangeStart(val),
		0x58 => Op::DesignatorRangeEnd(val),
		0x68 => Op::_Reserved(val),
		0x78 => Op::StringSingle(val),
		0x88 => Op::StringRangeStart(val),
		0x98 => Op::StringRangeEnd(val),
		0xA8 => Op::Delimiter,
		0xFC => Op::LongItem(val),
		_ => Op::Unk(id, val),
		}
	}
}

#[derive(Copy,Clone)]
pub struct InputFlags(u32);
impl_fmt!{
	Debug(self, f) for InputFlags {
		write!(f, "{:09b}", self.0)
	}
}
#[allow(dead_code)]
impl InputFlags
{
	pub fn is_constant(&self) -> bool {
		(self.0 & (1 << 0)) != 0
	}
	pub fn is_variable(&self) -> bool {
		(self.0 & (1 << 1)) != 0
	}
	pub fn is_relative(&self) -> bool {
		(self.0 & (1 << 2)) != 0
	}
	pub fn is_wrap(&self) -> bool {
		(self.0 & (1 << 3)) != 0
	}
}

#[derive(Default,Debug)]
pub struct ParseState
{
	// Global
	pub usage_page: u32,
	pub logical_range: (Option<i32>,Option<i32>),
	pub physical_range: (Option<i32>,Option<i32>),
	pub unit_exponent: Option<u32>,
	pub unit: Option<u32>,

	pub report_size: u32,
	pub report_id: Option<u32>,
	pub report_count: u32,

	// Local, cleared after the next main
	pub usage: List,
	pub designator: List,
	pub string: List,
}
#[derive(Debug)]
pub enum List
{
	Unset,
	Single(u32),	// TODO: What if there's multiple?
	Double([u32; 2]),
	ProtoRange(u32),
	Range(u32, u32),
}
impl Default for List {
	fn default() -> Self { List::Unset }
}
impl List
{
	fn set_single(&mut self, v: u32) {
		match *self
		{
		List::Unset => *self = List::Single(v),
		List::Single(p) => {
			*self = List::Double([p, v]);
			},
		_ => {},
		}
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
	
	/// Get value for the specified index
	pub fn get(&self, idx: usize) -> u32
	{
		match *self
		{
		List::Unset => 0,
		List::Single(v) => v,
		List::Double(l) => l[::core::cmp::min(1, idx)],
		List::ProtoRange(_v) => 0,
		List::Range(s,e) => {
			if idx <= (e - s) as usize {
				s + idx as u32
			}
			else {
				e
			}
			},
		}
	}
}
impl ParseState
{
	fn clear_local(&mut self)
	{
		self.usage = Default::default();
		self.designator = Default::default();
		self.string = Default::default();
	}
	/// Update state using the provided operation
	pub fn update(&mut self, op: Op)
	{
		match op
		{
		Op::Input(_) => { self.clear_local(); },
		Op::Output(_) => { self.clear_local(); },
		Op::Feature(_) => { self.clear_local(); },
		//Op::Collection(v) => self.collection.push(v),
		//Op::EndCollection => { self.collection.pop(); },
		Op::Collection(_) => {},
		Op::EndCollection => {},

		Op::UsagePage(v) => self.usage_page = v << 16,
		Op::LogicalMin(v) => self.logical_range.0 = Some(v),
		Op::LogicalMax(v) => self.logical_range.1 = Some(v),
		Op::PhysicalMin(v) => self.physical_range.0 = Some(v),
		Op::PhysicalMax(v) => self.physical_range.1 = Some(v),
		Op::UnitExponent(v) => self.unit_exponent = Some(v),
		Op::Unit(v) => self.unit = Some(v),
		Op::ReportSize(v) => self.report_size = v,
		Op::ReportId(v) => self.report_id = Some(v),
		Op::ReportCount(v) => self.report_count = v,

		Op::Push => todo!("push"),
		Op::Pop => todo!("pop"),

		Op::UsageSingle(v,is32) => self.usage.set_single( if is32 { 0 } else { self.usage_page } | v),
		Op::UsageRangeStart(v,is32) => self.usage.set_start(if is32 { 0 } else { self.usage_page } | v),
		Op::UsageRangeEnd(v,is32) => self.usage.set_end(if is32 { 0 } else { self.usage_page } | v),

		Op::DesignatorSingle(v) => self.designator.set_single(v),
		Op::DesignatorRangeStart(v) => self.designator.set_start(v),
		Op::DesignatorRangeEnd(v) => self.designator.set_end(v),

		Op::StringSingle(v) => self.string.set_single(v),
		Op::StringRangeStart(v) => self.string.set_start(v),
		Op::StringRangeEnd(v) => self.string.set_end(v),

		Op::Delimiter => todo!("Delimiter"),
		Op::_Reserved(..) => {},
		Op::LongItem(..) => {},
		Op::Unk(..) => {},
		}
	}
}

