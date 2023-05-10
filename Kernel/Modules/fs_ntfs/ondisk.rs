#![allow(dead_code)]
use crate::MftEntryIdx;
use ::kernel::prelude::*;

mod raw;

pub use self::raw::Bootsector;

pub struct Utf16Le([u8]);	// Note: has to account for non-alignment
impl Utf16Le {
	pub fn new(input: &[u8]) -> &Utf16Le {
		// SAFE: Same repr
		unsafe { ::core::mem::transmute(input) }
	}
	pub fn iter_units(&self) -> impl Iterator<Item=u16>+Clone+'_ {
		self.0.chunks(2).map(|v| u16::from_le_bytes([v[0], v[1]]))
	}
	pub fn chars(&self) -> impl Iterator<Item=char>+'_ {
		::utf16::Chars( self.iter_units() )
	}
}
impl ::core::cmp::PartialEq<str> for Utf16Le {
	fn eq(&self, s: &str) -> bool {
		self.chars().eq(s.chars())
	}
}
impl_fmt! {
	Debug(self,f) for Utf16Le {{
		write!(f, "w\"")?;
		for c in self.chars()
		{
			match c
			{
			'\\' => write!(f, "\\\\")?,
			'\n' => write!(f, "\\n")?,
			'\r' => write!(f, "\\r")?,
			'"' => write!(f, "\\\"")?,
			'\0' => write!(f, "\\0")?,
			// ASCII printable characters
			' '..='\u{127}' => write!(f, "{}", c)?,
			_ => write!(f, "\\u{{{:x}}}", c as u32)?,
			}
		}
		write!(f, "\"")?;
		Ok( () )
	}}
	Display(self,f) for Utf16Le {{
		for c in self.chars()
		{
			write!(f, "{}", c)?;
		}
		Ok( () )
	}}
}


pub const MFT_ENTRY_SELF: MftEntryIdx = MftEntryIdx(0);
pub const MFT_ENTRY_ROOT: MftEntryIdx = MftEntryIdx(5);

pub const ATTRNAME_DATA: &'static str = "";
pub const ATTRNAME_INDEXNAME: &'static str = "$I30";	// Index over attribute 0x30 (filename)

#[repr(u32)]
#[derive(Copy,Clone)]
pub enum FileAttr {
	StandardInformation = 0x10,
	FileName = 0x30,
	Data = 0x80,
	IndexRoot = 0x90,
	IndexAllocation = 0xA0,
	Bitmap = 0xB0,
}

pub struct MftEntry([u8]);
impl MftEntry {
	pub fn new_owned(v: ::kernel::vec::Vec<u8>) -> Box<MftEntry> {
		// SAFE: Same repr
		unsafe {
			::core::mem::transmute(v.into_boxed_slice())
		}
	}

	fn first_attrib_ofs(&self) -> u16 {
		raw::MftEntryHeader::first_attrib_ofs(&self.0).unwrap()
	}

	/// Iterate attributes
	pub fn iter_attributes(&self) -> impl Iterator<Item=&'_ MftAttrib> {
		MftEntryAttribs(&self.0[self.first_attrib_ofs() as usize..])
	}

	pub fn attr_handle<'s>(&'s self, a: &'s MftAttrib, idx: MftEntryIdx) -> AttrHandle {
		let s_s = self.0.as_ptr() as usize;
		let a_s = a.0.as_ptr() as usize;
		let a_e = a_s + a.0.len();
		let s_e = s_s + self.0.len();
		assert!(s_s <= a_s && a_s < s_e);
		assert!(s_s <= a_e && a_e <= s_e);
		AttrHandle(idx, a_s - s_s, a.0.len())
	}
}
pub struct AttrHandle(MftEntryIdx, usize, usize);

struct MftEntryAttribs<'a>(&'a [u8]);
impl<'a> Iterator for MftEntryAttribs<'a> {
	type Item = &'a MftAttrib;
	fn next(&mut self) -> Option<Self::Item> {
		use ::kernel::lib::byteorder::EncodedLE;
		if self.0.len() == 0 {
			return None;
		}
		if self.0.len() < 4 {
			// Inconsistent: Not enough space for a type flag
		}
		let ty: u32 = EncodedLE::decode(&mut &self.0[..4]).unwrap();
		if ty == !0 {
			// End of attributes marker
			return None;
		}
		if self.0.len() < 8 {
			// Inconsistent: Not enough space for an atribute header
			return None;
		}
		let size: u32 = EncodedLE::decode(&mut &self.0[4..8]).unwrap();
		if self.0.len() < size as usize {
			// Inconsistent: Over-sized attribute
			return None;
		}

		let rv = &self.0[..size as usize];
		self.0 = &self.0[size as usize..];
		Some(MftAttrib::new_borrowed(rv)?)
	}
}

pub struct MftAttrib([u8]);
impl MftAttrib {
	pub fn new_borrowed(v: &[u8]) -> Option<&MftAttrib> {
		if v.len() < ::core::mem::size_of::<raw::MftAttribHeader>() {
			return None;
		}
		if raw::MftAttribHeader::nonresident_flag(v).unwrap() != 0 {
			if v.len() - ::core::mem::size_of::<raw::MftAttribHeader>() < ::core::mem::size_of::<raw::MftAttrHeader_NonResident>() {
				return None;
			}
		}
		else {
			if v.len() - ::core::mem::size_of::<raw::MftAttribHeader>() < ::core::mem::size_of::<raw::MftAttrHeader_Resident>() {
				return None;
			}
		}
		// SAFE: Same repr
		Some(unsafe { ::core::mem::transmute(v) })
	}

	pub fn ty(&self) -> u32 {
		raw::MftAttribHeader::ty(&self.0).unwrap()
	}
	pub fn name(&self) -> &Utf16Le {
		let o = raw::MftAttribHeader::name_ofs(&self.0).unwrap();
		let l = raw::MftAttribHeader::name_length(&self.0).unwrap();
		Utf16Le::new(&self.0[o as usize..][..l as usize])
	}

	fn raw_data(&self) -> &[u8] {
		&self.0[ ::core::mem::size_of::<raw::MftAttribHeader>()..]
	}

	pub fn inner(&self) -> MftAttribData<'_> {
		if raw::MftAttribHeader::nonresident_flag(&self.0).unwrap() != 0 {
			// SAFE: Same repr
			MftAttribData::Nonresident(unsafe { ::core::mem::transmute(self.raw_data()) })
		}
		else {
			// SAFE: Same repr
			MftAttribData::Resident(unsafe { ::core::mem::transmute(self.raw_data()) })
		}
	}
}
pub enum MftAttribData<'a> {
	Nonresident(&'a MftAttribDataNonresident),
	Resident(&'a MftAttribDataResident),
}
pub struct MftAttribDataNonresident([u8]);
pub struct MftAttribDataResident([u8]);

