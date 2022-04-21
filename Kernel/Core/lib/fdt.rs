// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/lib/fdt.rs
//! "FDT" (Flattended Device Tree) parser
use crate::lib::byteorder::{ReadBytesExt,ByteOrder,BigEndian};

/// FDT parser/decoder structure
pub struct FDTRoot<'a>
{
	buffer: &'a [u8],
}

#[derive(Debug)]
enum Tag<'a>
{
	BeginNode(&'a str),
	EndNode,
	Prop(&'a str, &'a [u8]),
	Nop,
	End,
}

fn align_to_tag(v: usize) -> usize {
	(v + 4-1) & !(4-1)
}

impl<'a> FDTRoot<'a>
{
	/// Create a new FDT parser from a raw pointer to a FDT blob
	pub unsafe fn new_raw(base: *const u8) -> FDTRoot<'static> {
		log_trace!("FDTRoot::new_raw({:p})", base);
		let minbuf = ::core::slice::from_raw_parts(base, 8);
		let magic = BigEndian::read_u32(&minbuf[..4]);
		// TODO: Check magic value
		let len = BigEndian::read_u32(&minbuf[4..]);
		log_debug!("magic = {:#x}, len={:#x}", magic, len);
		
		Self::new_buf(::core::slice::from_raw_parts(base, len as usize))
	}
	/// Create a new FDT parser from a known slice of memory
	pub fn new_buf<'b>(buf: &'b [u8]) -> FDTRoot<'b> {
		let magic = BigEndian::read_u32(buf);
		assert_eq!(magic, 0xd00dfeed, "FDT magic mismatc - Expected 0xd00dfeed, got 0x{:8x}", magic);
		FDTRoot {
			buffer: buf,
		}
	}

	/// Shortcut for the physical address of the first page of the blocb
	pub fn phys(&self) -> crate::memory::PAddr {
		crate::memory::virt::get_phys(self.buffer.as_ptr())
	}
	pub fn size(&self) -> usize {
		self.buffer.len()
	}

	/// Dump the entire parsed tree to the logging stream
	pub fn dump_nodes(&self) {
		struct Indent(usize);
		impl ::core::fmt::Display for Indent { fn fmt(&self, f: &mut ::core::fmt::Formatter)->::core::fmt::Result { for _ in 0..self.0 { f.write_str(" ")?; } Ok(()) } }
		let mut ofs = 0;
		let mut indent = Indent(0);
		loop
		{
			let (tag, new_ofs) = self.next_tag(ofs);
			assert!(ofs < new_ofs);
			ofs = new_ofs;
			match tag
			{
			Tag::BeginNode(name) => {
				log_debug!("{}<{}>", indent, name);
				indent.0 += 1;
				},
			Tag::EndNode => {
				indent.0 -= 1;
				log_debug!("{}</>", indent);
				},
			Tag::Prop(name, data) => match name
				{
				"bootargs" |
				"stdout-path" |
				"device_type" |
				"clock-names" |
				"label" |
				"compatible"
					=> log_debug!("{}.{} = {:?}", indent, name, ::core::str::from_utf8(data)),
				"reg"
					=> if data.len() == 8+4 {
						let mut bytes = data;
						let a = bytes.read_u64::<BigEndian>().unwrap();
						let s = bytes.read_u32::<BigEndian>().unwrap();
						log_debug!("{}.{} = {:#x}+{:#x}", indent, name, a, s);
					}
					else if data.len() == 8+8 {
						let mut bytes = data;
						let a = bytes.read_u64::<BigEndian>().unwrap();
						let s = bytes.read_u64::<BigEndian>().unwrap();
						log_debug!("{}.{} = {:#x}+{:#x}", indent, name, a, s);
					}
					else {
						log_debug!("{}.{} = {:x?}", indent, name, data)
					},
				"phandle" | "interrupt-parent"
					=> if data.len() == 4 {
						let mut bytes = data;
						let v = bytes.read_u32::<BigEndian>().unwrap();
						log_debug!("{}.{} = {:#x}", indent, name, v);
					}
					else {
						log_debug!("{}.{} = 0x{:x?}", indent, name, data)
					},
				"timebase-frequency"
					=> if data.len() == 4 {
						let mut bytes = data;
						let f = bytes.read_u32::<BigEndian>().unwrap();
						log_debug!("{}.{} = {}", indent, name, f);
					}
					else {
						log_debug!("{}.{} = 0x{:x?}", indent, name, data)
					},
				_ => log_debug!("{}.{} = 0x{:x?}", indent, name, data),
				},
			Tag::End => break,
			_ => {},
			}
		}
	}

	pub fn items(&self) -> SubNodes {
		SubNodes {
			fdt: self,
			offset: 0,
			}
	}

	/// Return all immediate child nodes of the passed path
	pub fn get_nodes<'s,'p>(&'s self, path: &'p [&'p str]) -> NodesIter<'s, 'a, 'p> {
		NodesIter {
			fdt: self,
			path: path,
			offset: 0,
			path_depth: 0,
			cur_depth: 0,
		}
	}
	/// Return all properties matching the passed path
	pub fn get_props<'s,'p>(&'s self, path: &'p [&'p str]) -> PropsIterCb<'s, 'a, impl FnMut(usize, bool, &str)->bool + 'p> {
		PropsIterCb::new(self, move |ofs, is_leaf, name| ofs < path.len() && name == path[ofs] && is_leaf == (ofs == path.len() - 1))
	}
	/// Return all properties matching the provided callback
	/// 
	/// The callback recieves:
	/// - Current tree depth
	/// - If the checked node is a branch or a leaf
	/// - The node name
	pub fn get_props_cb<'s, F: FnMut(usize, bool, &str)->bool>(&'s self, cb: F) -> PropsIterCb<'s, 'a, F> {
		PropsIterCb::new(self, cb)
	}

	#[cfg(false_)]
	pub fn walk(&self, mut w: impl Walker<'a>)
	{
		let mut ofs = 0;
		loop
		{
			let (tag, new_ofs) = self.next_tag(ofs);
			assert!(ofs < new_ofs);
			ofs = new_ofs;
			match tag
			{
			Tag::Nop => {},
			Tag::BeginNode(name) => w.enter(Node { fdt: self, offset: ofs, name }),
			Tag::EndNode => w.leave(),
			Tag::Prop(name, data) => w.prop(name, data),
			Tag::End => break,
			}
		}
	}
}

#[cfg(false_)]
/// Trait allowing easy recursive traversal of the tree
pub trait Walker<'a>
{
	fn enter(&mut self, node: Node<'_, 'a>);
	fn leave(&mut self);
	fn prop(&mut self, name: &'a str, value: &'a [u8]);
}

// Internal helper methods
impl<'a> FDTRoot<'a>
{
	fn off_dt_struct(&self) -> usize {
		BigEndian::read_u32(&self.buffer[8..]) as usize
	}
	fn off_dt_strings(&self) -> usize {
		BigEndian::read_u32(&self.buffer[12..]) as usize
	}

	fn next_tag(&self, ofs: usize) -> (Tag<'a>, usize) {
		assert!(ofs % 4 == 0);
		let data = &self.buffer[self.off_dt_struct() + ofs .. ];
		let tag = BigEndian::read_u32(data);

		//log_trace!("tag = {}", tag);
		let data = &data[4..];
		match tag
		{
		0x1 => {	// FDT_BEGIN_NODE
			// NUL-terminated name follows
			let s = Self::get_nul_string(data);
			(Tag::BeginNode(s), ofs + 4 + align_to_tag(s.len()+ 1))
			},
		0x2 => {	// FDT_END_NODE
			(Tag::EndNode, ofs + 4)
			},
		0x3 => {	// FDT_PROP
			let len = BigEndian::read_u32(data) as usize;
			let name_ofs = BigEndian::read_u32(&data[4..]) as usize;
			let name = Self::get_nul_string(&self.buffer[self.off_dt_strings() + name_ofs..]);
			(Tag::Prop(name, &data[2*4..][..len]), ofs + 3 * 4 + align_to_tag(len))
			},
		0x4 => {	// FDT_NOP
			(Tag::Nop, ofs + 4)
			},
		0x9 => {	// FDT_END
			(Tag::End, ofs + 4)
			},
		_ => panic!("Unknown tag value {}", tag),
		}
	}

	fn get_nul_string(data: &[u8]) -> &str {
		let slen = data/*[..usize::min(data.len(), 256)]*/.iter().position(|x| *x == 0).expect("TODO: Handle unexpeted end in FDT");
		let s = ::core::str::from_utf8( &data[..slen] ).expect("TODO: Handle bad UTF-8 in FDT");
		s
	}
}

/// Iterator over a FDT using a checking callback
pub struct PropsIterCb<'a,'fdt: 'a, F> {
	fdt: &'a FDTRoot<'fdt>,
	offset: usize,
	cur_depth: u8,
	path_depth: u8,
	cb: F,
}
impl<'a, 'fdt: 'a, F> PropsIterCb<'a, 'fdt, F>
where
	F: FnMut(usize, bool, &str) -> bool,
{
	pub fn new(fdt: &'a FDTRoot<'fdt>, cb: F) -> Self
	{
		PropsIterCb {
			fdt,
			offset: 0,
			cur_depth: 0,
			path_depth: 0,
			cb
			}
	}
}
impl<'a, 'fdt: 'a, F> Iterator for PropsIterCb<'a, 'fdt, F>
where
	F: FnMut(usize, bool, &str) -> bool,
{
	type Item = &'fdt [u8];
	fn next(&mut self) -> Option<Self::Item>
	{
		loop
		{
			let (tag, next_ofs) = self.fdt.next_tag(self.offset);
			self.offset = next_ofs;
			match tag
			{
			Tag::BeginNode(name) => {
				if self.path_depth == self.cur_depth && (self.cb)(self.path_depth as usize, false, name) {
					// Increment both path and cur depth
					self.path_depth += 1;
				}
				self.cur_depth += 1;
				},
			Tag::EndNode => {
				if self.path_depth == self.cur_depth {
					assert!(self.path_depth > 0);
					self.path_depth -= 1;
				}
				self.cur_depth -= 1;
				},
			Tag::Prop(name, data) => {
				if self.path_depth == self.cur_depth && (self.cb)(self.path_depth as usize, true, name) {
					// Desired property
					return Some(data);
				}
				},
			Tag::End => return None,
			Tag::Nop => {},
			}
		}
	}
}

/// Iterator over a FDT's node using a fixed path
pub struct NodesIter<'a,'fdt: 'a,'b> {
	fdt: &'a FDTRoot<'fdt>,
	path: &'b [&'b str],
	offset: usize,
	path_depth: u8,
	cur_depth: u8,
}
impl<'a,'fdt, 'b> Iterator for NodesIter<'a, 'fdt, 'b>
{
	type Item = Node<'a,'fdt>;
	fn next(&mut self) -> Option<Self::Item>
	{
		// Last item in self.path is the property name
		let path_nodes_len = self.path.len() as u8;

		loop
		{
			let (tag, next_ofs) = self.fdt.next_tag(self.offset);
			self.offset = next_ofs;
			match tag
			{
			Tag::BeginNode(name) => {
				//log_trace!("BeginNode name = '{}' ({},{}) < {}", name, self.path_depth, self.cur_depth, path_nodes_len);
				if self.path_depth == self.cur_depth && self.path_depth < path_nodes_len {
					//log_trace!(" - '{}' == '{}'", name, self.path[self.path_depth as usize]);
					if name == self.path[self.path_depth as usize] {
						// Increment both path and cur depth
						self.path_depth += 1;
					}
				}
				self.cur_depth += 1;
				if self.path_depth + 1 == self.cur_depth && self.path_depth == path_nodes_len {
					return Some(Node { fdt: self.fdt, offset: self.offset, name: name });
				}
				},
			Tag::EndNode => {
				//log_trace!("EndNode ({},{})", self.path_depth, self.cur_depth);
				if self.path_depth == self.cur_depth {
					assert!(self.path_depth > 0);
					self.path_depth -= 1;
				}
				self.cur_depth -= 1;
				},
			Tag::Prop(..) => { },
			Tag::End => return None,
			Tag::Nop => {},
			}
		}
	}
}

/// A single FDT node
pub struct Node<'a, 'fdt: 'a>
{
	fdt: &'a FDTRoot<'fdt>,
	offset: usize,
	name: &'fdt str,
}
impl<'a, 'fdt: 'a> Node<'a, 'fdt>
{
	pub fn offset(&self) -> usize { self.offset }
	pub fn name(&self) -> &'fdt str { self.name }

	pub fn items(&self) -> SubNodes<'a, 'fdt> {
		SubNodes {
			fdt: self.fdt,
			offset: self.offset,
		}
	}

	pub fn get_prop(&self, name: &str) -> Option<&'fdt [u8]> {
		self.items()
			.filter_map( |(n, val)| if n == name { if let Item::Prop(p) = val { Some(p) } else { None } } else { None } )
			.next()
	}
}

pub enum Item<'a, 'fdt: 'a> {
	Node(Node<'a, 'fdt>),
	Prop(&'fdt [u8]),
}
/// Iterator over sub-nodes of a FDT
pub struct SubNodes<'a, 'fdt: 'a>
{
	fdt: &'a FDTRoot<'fdt>,
	offset: usize,
}
impl<'a, 'fdt: 'a> Iterator for SubNodes<'a, 'fdt> {
	type Item = (&'fdt str, Item<'a, 'fdt>);
	fn next(&mut self) -> Option<Self::Item>
	{
		let (tag, next_ofs) = self.fdt.next_tag(self.offset);
		self.offset = next_ofs;
		match tag
		{
		Tag::BeginNode(name) => {
			let rv = Node { fdt: self.fdt, offset: self.offset, name: name };
			let mut level = 0;
			loop {
				let (tag, next_ofs) = self.fdt.next_tag(self.offset);
				self.offset = next_ofs;
				match tag
				{
				Tag::BeginNode(_) => level += 1,
				Tag::EndNode =>
					if level == 0 {
						break;
					}
					else {
						level -= 1;
					},
				Tag::End => break,
				_ => {},
				}
			}
			Some( (name, Item::Node(rv)) )
			},
		Tag::EndNode => None,
		Tag::Prop(name, value) => Some( (name, Item::Prop(value)) ),
		Tag::End => None,
		Tag::Nop => Some( ("", Item::Prop(&[])) ),
		}
	}
}
