#![allow(dead_code)]
use crate::MftEntryIdx;
use ::kernel::prelude::*;

mod raw;

pub use self::raw::Bootsector;

macro_rules! delegate {
	($name:ident => $( $p:vis $field:ident: $t:ty ),* $(,)?) => {
		impl $name {
			$($p fn $field(&self) -> $t { raw::$name::$field(&self.0).unwrap() })*
		}
	}
}

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
	pub fn wtf8(&self) -> impl Iterator<Item=u8>+'_ {
		::utf16::Wtf8::new(self.chars())
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
	pub fn flags_isused(&self) -> bool {
		raw::MftEntryHeader::flags(&self.0).unwrap() & 0x1 != 0
	}
	pub fn flags_isdir(&self) -> bool {
		raw::MftEntryHeader::flags(&self.0).unwrap() & 0x2 != 0
	}

	/// Iterate attributes
	pub fn iter_attributes(&self) -> impl Iterator<Item=&'_ MftAttrib> {
		MftEntryAttribs(&self.0[self.first_attrib_ofs() as usize..])
	}

	pub fn attr_handle<'s>(&'s self, a: &'s MftAttrib) -> AttrHandle {
		let s_s = self.0.as_ptr() as usize;
		let a_s = a.0.as_ptr() as usize;
		let a_e = a_s + a.0.len();
		let s_e = s_s + self.0.len();
		assert!(s_s <= a_s && a_s < s_e);
		assert!(s_s <= a_e && a_e <= s_e);
		AttrHandle(a_s - s_s, a.0.len())
	}

	pub fn get_attr(&self, handle: &AttrHandle) -> Option<&MftAttrib> {
		MftAttrib::new_borrowed(self.0.get(handle.0..)?.get(..handle.1)?)
	}
}
pub struct AttrHandle(usize, usize);

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
		let o = raw::MftAttribHeader::name_ofs(&self.0).unwrap();	// Byte offset
		let l = raw::MftAttribHeader::name_length(&self.0).unwrap();	// Number of u16s
		Utf16Le::new(&self.0[o as usize..][..l as usize*2])
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
impl<'a> MftAttribData<'a> {
	pub fn as_resident(&self) -> Option<&'a MftAttribDataResident> {
		match *self {
		MftAttribData::Resident(v) => Some(v),
		_ => None,
		}
	}
}
pub struct MftAttribDataNonresident([u8]);
impl MftAttribDataNonresident {
	pub fn starting_vcn(&self) -> u64 { raw::MftAttrHeader_NonResident::starting_vcn(&self.0).unwrap() }
	pub fn last_vcn(&self) -> u64 { raw::MftAttrHeader_NonResident::last_vcn(&self.0).unwrap() }

	pub fn data_run_ofs(&self) -> u16 { raw::MftAttrHeader_NonResident::data_run_ofs(&self.0).unwrap() }
	pub fn compression_unit_size(&self) -> u16 { raw::MftAttrHeader_NonResident::compression_unit_size(&self.0).unwrap() }
	pub fn allocated_size(&self) -> u64 { raw::MftAttrHeader_NonResident::allocated_size(&self.0).unwrap() }
	pub fn real_size(&self) -> u64 { raw::MftAttrHeader_NonResident::real_size(&self.0).unwrap() }
	pub fn initiated_size(&self) -> u64 { raw::MftAttrHeader_NonResident::initiated_size(&self.0).unwrap() }

	pub fn data_runs(&self) -> impl Iterator<Item=DataRun> + '_ {
		struct It<'a>(&'a [u8], u64);
		impl<'a> Iterator for It<'a> {
			type Item = DataRun;
			fn next(&mut self) -> Option<Self::Item> {
				if self.0.len() == 0 {
					None
				}
				else {
					let lens = self.0[0];
					let size_len = (lens & 0xF) as usize;
					let size_ofs = (lens >> 4) as usize;
					// If the size of the runlength is zero, assume end of the list
					if size_len == 0 {
						self.0 = &[];
						return None;
					}
					let rundesc_len = 1 + size_len + size_ofs;
					// BUGCHECK: There must be enough data
					if self.0.len() < rundesc_len {
						self.0 = &[];
						return None;
					}
					let len = &self.0[1..][..size_len];
					let ofs = &self.0[1+size_len..][..size_ofs];
					// Sanity checks: Only support 64 bit offsets and 32 bit counts
					if size_len > 8 {
						self.0 = &[];
						return None;
					}
					if size_ofs > 8 {
						self.0 = &[];
						return None;
					}

					fn parse_int(bytes: &[u8], sign_extend: bool) -> u64 {
						let mut rv = 0;
						for (i,b) in bytes.iter().enumerate() {
							rv |= (*b as u64) << (i*8);
						}
						if sign_extend && bytes.len() < 8 && bytes.last().unwrap_or(&0) & 0x80 != 0 {
							rv |= !0 << (bytes.len() * 8);
						}
						rv
					}
					let len = parse_int(len, false);
					let ofs = parse_int(ofs, true);	// Offset is signed, it's relative to the last entry
					let lcn = self.1 + ofs;
					self.0 = &self.0[rundesc_len..];
					self.1 = lcn;
					Some(DataRun { 
						lcn: lcn,
						cluster_count: len,
					})
				}
			}
		}
		// `data_run_ofs` is relative to the start of the attribute, so offset by the size of the header
		let ofs = self.data_run_ofs() as usize;
		let Some(ofs) = ofs.checked_sub(4*4) else { return It(&[], 0); };
		It(&self.0[ofs..], 0)
	}
}
pub struct DataRun {
	/// Logical cluster number - i.e. cluster index relative to the start of the filesystem
	pub lcn: u64,
	/// Number of sequential clusters in this run
	pub cluster_count: u64,
}

pub struct MftAttribDataResident([u8]);
impl MftAttribDataResident {
	pub fn indexed(&self) -> bool {
		raw::MftAttrHeader_Resident::indexed_flag(&self.0).unwrap() != 0
	}
	pub fn data(&self) -> &[u8] {
		let ofs = raw::MftAttrHeader_Resident::attrib_ofs(&self.0).unwrap() as usize;
		let len = raw::MftAttrHeader_Resident::attrib_len(&self.0).unwrap() as usize;
		let Some(ofs) = ofs.checked_sub(4*4) else { return &[]; };
		self.0.get(ofs..).and_then(|v| v.get(..len)).unwrap_or(&[])
	}
}

pub struct Attrib_IndexRoot([u8]);
impl Attrib_IndexRoot {
	pub fn from_slice(v: &[u8]) -> Option<&Self> {
		if v.len() < ::core::mem::size_of::<raw::Attrib_IndexRoot>() {
			return None;
		}
		// SAFE: Same repr
		let rv: &Self = unsafe { ::core::mem::transmute(v) };
		Attrib_IndexHeader::from_slice(&rv.index_header_bytes())?;
		Some(rv)
	}

	fn index_header_bytes(&self) -> &[u8] {
		&self.0[16..]
	}
	pub fn index_header(&self) -> &Attrib_IndexHeader {
		Attrib_IndexHeader::from_slice(self.index_header_bytes()).unwrap()
	}
}
delegate!{ Attrib_IndexRoot =>
	pub index_block_size: u32,
}
pub struct Attrib_IndexHeader([u8]);
impl Attrib_IndexHeader {
	fn size_of() -> usize {
		::core::mem::size_of::<raw::Attrib_IndexHeader>()
	}
	pub fn from_slice(v: &[u8]) -> Option<&Self> {
		if v.len() < Self::size_of() {
			return None;
		}
		// SAFE: Same repr
		let rv: &Self = unsafe { ::core::mem::transmute(v) };
		if (rv.first_entry_offset() as usize) < Self::size_of() {
			return None;
		}
		if rv.first_entry_offset() as usize > v.len() {
			return None;
		}
		Some(rv)
	}

	pub fn entries_slice(&self) -> &[u8] {
		&self.0[self.first_entry_offset() as usize..]
	}
}
delegate!{ Attrib_IndexHeader =>
	pub flags: u8,
	first_entry_offset: u32,
}

/// Header on a index block (within an INDEX_ALLOCATION)
pub struct Attrib_IndexBlockHeader([u8]);
impl Attrib_IndexBlockHeader {
	pub fn from_slice(v: &[u8]) -> Option<&Self> {
		if v.len() < ::core::mem::size_of::<raw::Attrib_IndexBlockHeader>() {
			return None;
		}
		// SAFE: Same repr
		let rv: &Self = unsafe { ::core::mem::transmute(v) };
		if !(rv.magic() == 0x58_44_4e_49) {
			return None;
		}
		Attrib_IndexHeader::from_slice(&rv.index_header_bytes())?;
		Some(rv)
	}

	fn index_header_bytes(&self) -> &[u8] {
		&self.0[0x18..]
	}
	pub fn index_header(&self) -> &Attrib_IndexHeader {
		Attrib_IndexHeader::from_slice(self.index_header_bytes()).unwrap()
	}
	//pub fn update_sequence(&self) -> &[u8] {
	//}
}
delegate!{ Attrib_IndexBlockHeader =>
	magic: u32,
	update_sequence_ofs: u16,
	update_sequence_size: u16,

	pub log_file_sequence_number: u64,
	pub this_vcn: u64,
}

pub struct Attrib_IndexEntry([u8]);
impl Attrib_IndexEntry {
	fn size_of() -> usize {
		::core::mem::size_of::<raw::Attrib_IndexEntry>()
	}
	pub fn from_slice(v: &[u8]) -> Option<&Self> {
		if v.len() < Self::size_of() {
			log_debug!("Attrib_IndexEntry::from_slice: Too small, {} < {}", v.len(), Self::size_of());
			return None;
		}
		// SAFE: Same repr
		let rv: &Self = unsafe { ::core::mem::transmute(v) };
		//log_debug!("Attrib_IndexEntry::data: {}+{}={}", Self::size_of(), rv.message_len(), rv.entry_size());
		if (rv.entry_size() as usize) < Self::size_of() {
			return None;
		}
		if rv.entry_size() as usize > v.len() {
			return None;
		}
		if rv.message_len() as usize > rv.tail().len() {
			return None;
		}
		Some(rv)
	}
	fn tail(&self) -> &[u8] {
		&self.0[Self::size_of()..self.entry_size() as usize]
	}
	pub fn data(&self) -> &[u8] {
		&self.tail()[..self.message_len() as usize]
	}
	pub fn next(&self) -> Option<&[u8]> {
		if self.index_flags() & 0x02 == 0 {
			Some( &self.0[self.entry_size() as usize..] )
		}
		else {
			None
		}
	}
	pub fn subnode_vcn(&self) -> Option<u64> {
		if self.index_flags() & 0x01 == 0 {
			None
		}
		else {
			let b = &self.0[self.entry_size() as usize - 8..][..8];
			Some( <::kernel::lib::byteorder::LittleEndian as ::kernel::lib::byteorder::ByteOrder>::read_u64(b) )
		}
	}

	pub fn mft_reference_num(&self) -> u64 {
		self.mft_reference() & ((1<<48)-1)
	}
	pub fn mft_reference_seq(&self) -> u16 {
		(self.mft_reference() >> 48) as u16
	}
}
delegate! { Attrib_IndexEntry =>
	pub mft_reference: u64,
	message_len: u16,
	entry_size: u16,
	pub index_flags: u16,
}


pub struct Attrib_Filename([u8]);
impl Attrib_Filename {
	fn size_of() -> usize {
		// NOTE: The structure's length is not aligned (two trailing bytes for the filename length and namespace
		//::core::mem::size_of::<raw::Attrib_Filename>()
		0x42
	}
	pub fn from_slice(v: &[u8]) -> Option<&Self> {
		if v.len() < Self::size_of() {
			log_debug!("Attrib_Filename::from_slice: Too small, {} < {}", v.len(), Self::size_of());
			return None;
		}
		// SAFE: Same repr
		let rv: &Self = unsafe { ::core::mem::transmute(v) };
		let full_size = Self::size_of() + rv.filename_length() as usize * 2;
		if v.len() < full_size {
			log_debug!("Attrib_Filename::from_slice: Too small, {} < {}", v.len(), full_size);
			return None;
		}
		Some(rv)
	}
	pub fn filename(&self) -> &Utf16Le {
		let d = &self.0[Self::size_of()..][..self.filename_length() as usize * 2];
		Utf16Le::new(d)
	}
}
delegate!{ Attrib_Filename =>
	filename_length: u8,
}
