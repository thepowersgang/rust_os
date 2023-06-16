#![allow(dead_code)]
#![allow(non_camel_case_types)]
use crate::MftEntryIdx;

mod raw;

pub use self::raw::Bootsector;

macro_rules! delegate {
	($name:ident => $( $(#[$m:meta])* $p:vis $field:ident: $t:ty ),* $(,)?) => {
		delegate!{$name -> $name => $( $(#[$m])* $p $field: $t),*}
	};
	($name:ident -> $name2:ident => $( $(#[$m:meta])* $p:vis $field:ident: $t:ty ),* $(,)?) => {
		impl $name {
			$( $(#[$m])* $p fn $field(&self) -> $t { raw::$name2::$field(&self.0).unwrap() })*
		}
	};
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

// TODO: Use the update sequence to fix loaded items (contains the actual values of each sector's last two bytes)
pub struct UpdateSequence([u8]);
impl UpdateSequence {
	pub fn new_borrowed(v: &[u8]) -> Option<&Self> {
		if v.len() < 2 {
			return None;
		}
		if v.len() % 2 != 0 {
			return None;
		}
		// SAFE: Same repr
		Some(unsafe { ::core::mem::transmute(v) })
	}
	fn from_subslice(v: &[u8], ofs: u16, num_words: u16) -> Option<&Self> {
		if ofs as usize >= v.len() {
			return None;
		}
		let v = v.get(ofs as usize..)?;
		let nbytes = num_words as usize * 2;
		let v = v.get(..nbytes)?;
		Self::new_borrowed(v)
	}
	pub fn sequence_number(&self) -> u16 {
		u16::from_le_bytes([self.0[0], self.0[1]])
	}
	pub fn array(&self) -> impl Iterator<Item=u16>+'_ {
		struct IterU16<'a>(&'a [u8]);
		impl<'a> Iterator for IterU16<'a> {
			type Item = u16;
			fn next(&mut self) -> Option<Self::Item> {
				match ::kernel::lib::split_off_front(&mut self.0, 2)
				{
				None => None,
				Some(v) => Some( u16::from_le_bytes([v[0], v[1]]) ),
				}
			}
		}
		IterU16(&self.0[2..])
	}
}

pub struct MftEntry([u8]);
delegate!{ MftEntry -> MftEntryHeader =>
	first_attrib_ofs: u16,
	flags: u16,

	update_sequence_ofs: u16,
	update_sequence_size: u16,
}
impl MftEntry {
	pub fn new_borrowed(v: &[u8]) -> Option<&Self> {
		if v.len() < ::core::mem::size_of::<raw::MftEntryHeader>() {
			return None;
		}
		// SAFE: Same repr
		let rv: &Self = unsafe { ::core::mem::transmute(v) };
		if rv.first_attrib_ofs() as usize >= v.len() {
			return None;
		}
		UpdateSequence::from_subslice(v, rv.update_sequence_ofs(), rv.update_sequence_size())?;
		Some(rv)
	}

	pub fn flags_isused(&self) -> bool {
		self.flags() & 0x1 != 0
	}
	pub fn flags_isdir(&self) -> bool {
		self.flags() & 0x2 != 0
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

	pub fn update_sequence(&self) -> &UpdateSequence {
		UpdateSequence::from_subslice(&self.0, self.update_sequence_ofs(), self.update_sequence_size()).unwrap()
	}
}
/// Saved handle (offset+size) to an attribute
pub struct AttrHandle(usize, usize);

/// Iterator over attributes in a MFT entry
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
			// End of attributes marker - clean
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
		//log_debug!("MftEntryAttribs::next(): {:?}", ::kernel::logging::HexDump(rv));
		self.0 = &self.0[size as usize..];
		Some(MftAttrib::new_borrowed(rv)?)
	}
}

pub struct MftAttrib([u8]);
delegate!{MftAttrib -> MftAttrHeader =>
	pub ty: u32,
	size: u32,
	nonresident_flag: u8,
	name_length: u8,
	name_ofs: u16,
	pub flags: u16,
	pub attribute_id: u16,
}
impl MftAttrib {
	fn size_of() -> usize {
		::core::mem::size_of::<raw::MftAttrHeader>()
	}
	pub fn new_borrowed(v: &[u8]) -> Option<&Self> {
		if v.len() < Self::size_of() {
			log_error!("MftAttr: Too small {} < {}", v.len(), Self::size_of());
			return None;
		}
		// SAFE: Same repr
		let rv: &Self = unsafe { ::core::mem::transmute(v) };
		if rv.name_ofs() as usize > v.len() {
			return None;
		}
		if rv.name_ofs() as usize + rv.name_length() as usize * 2 > v.len() {
			return None;
		}
		if rv.nonresident_flag() != 0 {
			MftAttrHeader_NonResident::from_slice(rv.raw_data())?;
		}
		else {
			MftAttrHeader_Resident::from_slice(rv.raw_data())?;
		}
		Some(rv)
	}

	pub fn name(&self) -> &Utf16Le {
		let o = self.name_ofs() as usize;	// Byte offset
		let l = self.name_length() as usize * 2;	// Number of u16s
		Utf16Le::new(&self.0[o..][..l])
	}

	fn raw_data(&self) -> &[u8] {
		&self.0[ Self::size_of() .. ]
	}

	pub fn inner(&self) -> MftAttribData<'_> {
		if self.nonresident_flag() != 0 {
			MftAttribData::Nonresident(MftAttrHeader_NonResident::from_slice(self.raw_data()).unwrap())
		}
		else {
			MftAttribData::Resident(MftAttrHeader_Resident::from_slice(self.raw_data()).unwrap())
		}
	}
}
pub enum MftAttribData<'a> {
	Nonresident(&'a MftAttrHeader_NonResident),
	Resident(&'a MftAttrHeader_Resident),
}
impl<'a> MftAttribData<'a> {
	pub fn as_resident(&self) -> Option<&'a MftAttrHeader_Resident> {
		match *self {
		MftAttribData::Resident(v) => Some(v),
		_ => None,
		}
	}
}
pub struct MftAttrHeader_NonResident([u8]);
delegate!{ MftAttrHeader_NonResident =>
	pub starting_vcn: u64,
	pub last_vcn: u64,
	data_run_ofs: u16,
	/// Power-of-two number of clusters in a compression unit
	pub compression_unit_size: u16,
	/// Data size, rounded up to clusters
	pub allocated_size: u64,
	/// Size of the data (bytes)
	pub real_size: u64,
	/// Size of data on-disk? (is this something other than the allocated size?)
	pub initiated_size: u64,
}
impl MftAttrHeader_NonResident {
	fn size_of() -> usize {
		::core::mem::size_of::<raw::MftAttrHeader_NonResident>()
	}
	fn from_slice(v: &[u8]) -> Option<&Self> {
		if v.len() < Self::size_of() {
			log_error!("MftAttrHeader_NonResident: Too small {} < {}", v.len(), Self::size_of());
			return None;
		}
		// SAFE: Same repr
		let rv: &Self = unsafe { ::core::mem::transmute(v) };
		if rv.data_run_ofs() < 16 {
			return None;
		}
		if rv.data_run_ofs() as usize - 16 > v.len() {
			return None;
		}
		Some(rv)
	}
	/// Iterator over the data on-disk
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
					let lcn = if ofs.len() > 0 {
							let ofs = parse_int(ofs, true);
							let lcn = self.1.wrapping_add(ofs);	// Offset is signed, it's relative to the last entry
							self.1 = lcn;
							Some(lcn)
						} else {
							None
						};
					self.0 = &self.0[rundesc_len..];
					Some(DataRun { 
						lcn,
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
	pub lcn: Option<u64>,
	/// Number of sequential clusters in this run
	pub cluster_count: u64,
}

pub struct MftAttrHeader_Resident([u8]);
delegate!{ MftAttrHeader_Resident =>
	attrib_len: u32,
	attrib_ofs: u16,
	indexed_flag: u8,
}
impl MftAttrHeader_Resident {
	fn size_of() -> usize {
		::core::mem::size_of::<raw::MftAttrHeader_Resident>()
	}
	fn from_slice(v: &[u8]) -> Option<&Self> {
		if v.len() < Self::size_of() {
			log_error!("MftAttrHeader_Resident: Too small {} < {}", v.len(), Self::size_of());
			return None;
		}
		// SAFE: Same repr
		let rv: &Self = unsafe { ::core::mem::transmute(v) };
		//log_debug!("{}+{}", rv.attrib_ofs(), rv.attrib_len());
		if rv.attrib_ofs() < 4*4 {
			log_error!("MftAttrHeader_Resident: attrib_ofs({}) too small (must be at least 16)", rv.attrib_ofs());
			return None;
		}
		if rv.adj_attrib_ofs() > v.len() {
			log_error!("MftAttrHeader_Resident: adj_attrib_ofs({}) out of bounds (>{})", rv.adj_attrib_ofs(), v.len());
			return None;
		}
		if rv.adj_attrib_ofs() + rv.attrib_len() as usize > v.len() {
			log_error!("MftAttrHeader_Resident: adj_attrib_ofs({})+attrib_len({}) out of bounds (>{})", rv.adj_attrib_ofs(), rv.attrib_len(), v.len());
			return None;
		}
		Some(rv)
	}
	pub fn indexed(&self) -> bool {
		self.indexed_flag() != 0
	}
	fn adj_attrib_ofs(&self) -> usize {
		// Adjust for the size of the base header (16 bytes)
		self.attrib_ofs() as usize - 16
	}
	pub fn data(&self) -> &[u8] {
		let ofs = self.adj_attrib_ofs();
		let len = self.attrib_len() as usize;
		&self.0[ofs..][..len]
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
	// TODO: the update sequence handles spotting badly-written sectors
	pub fn update_sequence(&self) -> &UpdateSequence {
		UpdateSequence::from_subslice(&self.0, self.update_sequence_ofs(), self.update_sequence_size()).unwrap()
	}
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
