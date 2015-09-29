//
//
//
use ::lib::byteorder::{ByteOrder,BigEndian};

pub struct FDTRoot<'a>
{
	buffer: &'a [u8],
}

#[repr(C)]
struct BE32(u32);

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
		let minbuf = ::core::slice::from_raw_parts(base, 8);
		let magic = BigEndian::read_u32(&minbuf[..4]);
		let len = BigEndian::read_u32(&minbuf[4..]);
		log_debug!("magic = {:#x}, len={:#x}", magic, len);
		
		Self::new_buf(::core::slice::from_raw_parts(base, len as usize))
	}
	pub fn new_buf<'b>(buf: &'b [u8]) -> FDTRoot<'b> {
		let magic = BigEndian::read_u32(buf);
		assert_eq!(magic, 0xd00dfeed);
		FDTRoot {
			buffer: buf,
		}
	}

	fn off_dt_struct(&self) -> usize {
		BigEndian::read_u32(&self.buffer[8..]) as usize
	}
	fn off_dt_strings(&self) -> usize {
		BigEndian::read_u32(&self.buffer[12..]) as usize
	}

	fn next_tag(&self, ofs: usize) -> (Tag, usize) {
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
				"bootargs" => log_debug!(".{} = {:?}", name, ::core::str::from_utf8(data)),
				"stdout-path" |
				"device_type" |
				"compatible"
					=> log_debug!(".{} = {:?}", name, ::core::str::from_utf8(data)),
				_ => log_debug!(".{} = {:?}", name, data),
				},
			Tag::End => break,
			_ => {},
			}
		}
	}
}


//struct NodesIter<'a>(FDTRoot<'a>, usize);
//impl NodesIter
//{
//}
