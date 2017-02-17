//
//
//
use ::lib::byteorder::{ByteOrder,BigEndian};

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
	pub unsafe fn new_raw(base: *const u8) -> FDTRoot<'static> {
		log_trace!("FDTRoot::new_raw({:p})", base);
		let minbuf = ::core::slice::from_raw_parts(base, 8);
		let magic = BigEndian::read_u32(&minbuf[..4]);
		let len = BigEndian::read_u32(&minbuf[4..]);
		log_debug!("magic = {:#x}, len={:#x}", magic, len);
		
		Self::new_buf(::core::slice::from_raw_parts(base, len as usize))
	}
	pub fn new_buf<'b>(buf: &'b [u8]) -> FDTRoot<'b> {
		let magic = BigEndian::read_u32(buf);
		assert_eq!(magic, 0xd00dfeed, "FDT magic mismatc - Expected 0xd00dfeed, got 0x{:8x}", magic);
		FDTRoot {
			buffer: buf,
		}
	}

	pub fn size(&self) -> usize {
		self.buffer.len()
	}

	pub fn dump_nodes(&self) {
		let mut ofs = 0;
		loop
		{
			let (tag, new_ofs) = self.next_tag(ofs);
			assert!(ofs < new_ofs);
			ofs = new_ofs;
			match tag
			{
			Tag::BeginNode(name) => log_debug!("<{}>", name),
			Tag::EndNode => log_debug!("</>"),
			Tag::Prop(name, data) => match name
				{
				"bootargs" |
				"stdout-path" |
				"device_type" |
				"clock-names" |
				"label" |
				"compatible"
					=> log_debug!(".{} = {:?}", name, ::core::str::from_utf8(data)),
				"reg" |
				"interrupts"
					=> if data.len() == 8+4 {
						use lib::byteorder::{ReadBytesExt,BigEndian};
						let mut bytes = data;
						let a = bytes.read_u64::<BigEndian>().unwrap();
						let s = bytes.read_u32::<BigEndian>().unwrap();
						log_debug!(".{} = {:#x}+{:#x}", name, a, s);
					}
					else if data.len() == 8+8 {
						use lib::byteorder::{ReadBytesExt,BigEndian};
						let mut bytes = data;
						let a = bytes.read_u64::<BigEndian>().unwrap();
						let s = bytes.read_u64::<BigEndian>().unwrap();
						log_debug!(".{} = {:#x}+{:#x}", name, a, s);
					}
					else {
						log_debug!(".{} = {:?}", name, data)
					},
				_ => log_debug!(".{} = {:?}", name, data),
				},
			Tag::End => break,
			_ => {},
			}
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
	pub fn get_props<'s,'p>(&'s self, path: &'p [&'p str]) -> PropsIter<'s, 'a, 'p> {
		PropsIter {
			fdt: self,
			path: path,
			offset: 0,
			path_depth: 0,
			cur_depth: 0,
		}
	}
}

impl<'a> FDTRoot<'a>
{
	fn off_dt_struct(&self) -> usize {
		BigEndian::read_u32(&self.buffer[8..]) as usize
	}
	fn off_dt_strings(&self) -> usize {
		BigEndian::read_u32(&self.buffer[12..]) as usize
	}

	fn next_tag(&self, ofs: usize) -> (Tag<'a>, usize) {
		//log_debug!("FDTRoot::next_tag(ofs={}) len={}", ofs, self.buffer.len());
		assert!(ofs % 4 == 0);
		let data = &self.buffer[self.off_dt_struct() + ofs .. ];
		let tag = BigEndian::read_u32(data);

		//log_trace!("tag = {}", tag);
		let data = &data[4..];
		match tag
		{
		0x1 => {	// FDT_BEGIN_NODE
			// NUL-terminated name follows
			let slen = data[..256].iter().position(|x| *x == 0).expect("TODO: Handle unexpeted end in FDT");
			let s = ::core::str::from_utf8( &data[..slen] ).expect("TODO: Handle bad UTF-8 in FDT");
			(Tag::BeginNode(s), ofs + 4 + align_to_tag(slen + 1))
			},
		0x2 => {	// FDT_END_NODE
			(Tag::EndNode, ofs + 4)
			},
		0x3 => {	// FDT_PROP
			let len = BigEndian::read_u32(data) as usize;
			let name_ofs = BigEndian::read_u32(&data[4..]) as usize;
			//log_trace!("len = {}, name_ofs = {}", len, name_ofs);
			let name_base = &self.buffer[self.off_dt_strings() + name_ofs..];
			let name_len = name_base[..256].iter().position(|x| *x == 0).expect("TODO: Handle unexpeted end in FDT");
			let name = ::core::str::from_utf8( &name_base[..name_len] ).expect("TODO: Handle bad UTF-8 in FDT");
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
}


pub struct PropsIter<'a,'fdt: 'a,'b> {
	fdt: &'a FDTRoot<'fdt>,
	path: &'b [&'b str],
	offset: usize,
	path_depth: u8,
	cur_depth: u8,
}
impl<'a,'fdt, 'b> Iterator for PropsIter<'a, 'fdt, 'b>
{
	type Item = &'fdt [u8];
	fn next(&mut self) -> Option<Self::Item>
	{
		// Last item in self.path is the property name
		let path_nodes_len = (self.path.len() - 1) as u8;

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
				},
			Tag::EndNode => {
				//log_trace!("EndNode ({},{})", self.path_depth, self.cur_depth);
				if self.path_depth == self.cur_depth {
					assert!(self.path_depth > 0);
					self.path_depth -= 1;
				}
				self.cur_depth -= 1;
				},
			Tag::Prop(name, data) => {
				//log_trace!("Prop name = '{}' ({},{}) == {}", name, self.path_depth, self.cur_depth, path_nodes_len);
				if self.path_depth == self.cur_depth && self.path_depth == path_nodes_len {
					//log_trace!(" - '{}' == '{}'", name, self.path[self.path_depth as usize]);
					if name == self.path[path_nodes_len as usize] {
						// Desired property
						return Some(data);
					}
				}
				},
			Tag::End => return None,
			Tag::Nop => {},
			}
		}
	}
}

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
