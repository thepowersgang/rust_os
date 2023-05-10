
macro_rules! pod_impls {
	($t:ty) => {
		impl Copy for $t {
		}
		impl Clone for $t {
			fn clone(&self) -> Self { *self }
		}
		unsafe impl ::kernel::lib::POD for $t {}
		impl Default for $t {
			fn default() -> Self {
				// SAFE: Copy types are safe to zero... well, except &, but meh
				unsafe { ::core::mem::zeroed() }
			}
		}
		impl $t {
			extern "C" fn _check_extern(_: &Self) {
			}
			#[allow(dead_code)]
			pub fn from_slice(r: &[u8]) -> Self {
				let mut rv = Self::default();
				::kernel::lib::as_byte_slice_mut(&mut rv).copy_from_slice(r);
				rv
			}
		}
	};
}

/// The first sector of a NTFS volume
#[repr(C, packed)]
pub struct Bootsector {
	jump_instr: [u8; 3],
	pub system_id: [u8; 8], // "NTFS	"
	pub bytes_per_sector: u16,
	pub sectors_per_cluster: u8,

	// Offset 0xe
	_unused_0x0e: [u8; 7],
	pub media_descriptor: u8,
	_unused_0x16: [u8; 2],
	pub sectors_per_track: u16,
	pub heads: u16,

	// Offset 0x1C
	_unused_0x1c: u64,
	unknown_0x24: u32,

	// Offset 0x28
	pub total_sector_count: u64,
	pub mft_start: u64,
	pub mft_mirror_start: u64,

	// Offset 0x40
	pub mft_record_size: RecordSizeVal,
	pub index_record_size: RecordSizeVal,
	pub serial_number: u64,

	_padding: [u8; 512 - 0x50],
}
pod_impls!(Bootsector);

/// A record size: If positive then it's a cluster count, if negative it's a byte size power of 2
pub struct RecordSizeVal(u8, [u8; 3]);
pod_impls!(RecordSizeVal);
impl RecordSizeVal {
	pub fn raw(&self) -> u8 { self.0 }
	pub fn get(&self) -> RecordSize {
		if self.0 < 0x80 {
			RecordSize::Clusters(self.0 as usize)
		} else {
			let shift = !self.0 + 1;
			RecordSize::Bytes(1 << shift)
		}
	}
}
pub enum RecordSize {
	Clusters(usize),
	Bytes(usize),
}
impl RecordSize {
	pub fn to_bytes(self, cluster_size: usize) -> usize {
		match self {
		RecordSize::Clusters(n) => n * cluster_size,
		RecordSize::Bytes(rv) => rv,
		}
	}
}

/// Aka 'FILE'
#[derive(::kernel_derives::FieldsLE)]
pub struct MftEntryHeader {
	pub magic: [u8; 4],

	pub update_sequence_ofs: u16,
	// Size in words of the UpdateSequenceArray
	pub update_sequence_size: u16,

	/// $LogFile Sequence Number
	pub lsn: u64,

	pub sequence_number: u16,
	pub hard_link_count: u16,
	pub first_attrib_ofs: u16, // Size of header?
	/// 0: In Use, 1: Directory
	pub flags: u16,

	/// Real Size of FILE Record
	pub record_size: u32,
	/// Allocated Size for FILE Record
	pub record_space: u32,

	/// Base address of the MFT containing this record
	/// "File reference to the base FILE record" ???
	pub reference: u64,

	pub next_attrib_id: u16,

	//pub osdep: MftEntryHeader_OsDep,
}
#[repr(C)]
pub union MftEntryHeader_OsDep {
	xp: MftEntryHeader_OsDep_Xp,
	all: MftEntryHeader_OsDep_All,
}


// Only in XP
#[repr(C)]
#[derive(Copy, Clone)]
pub struct MftEntryHeader_OsDep_Xp {
	align_to_4bytes: u16,
	/// Number of this MFT Record
	record_number: u16,
	update_sequence_number: u16,
	update_sequence_array: [u16; 0],
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct MftEntryHeader_OsDep_All {
	update_sequence_number: u16,
	update_sequence_array: [u16; 0],
}

#[derive(::kernel_derives::FieldsLE)]
#[repr(C)]
pub struct MftAttribHeader {
	ty: u32,	// See eNTFS_FILE_Attribs
	size: u32,	// Includes header

	nonresident_flag: u8,
	name_length: u8,
	name_ofs: u16,

	flags: u16,	// 0: Compressed, 14: Encrypted, 15: Sparse
	attribute_id: u16,
}
#[derive(::kernel_derives::FieldsLE)]
#[repr(C)]
pub struct MftAttrHeader_Resident {
	/// Length of the attribute data in WORDS
	attrib_len: u32,
	// TODO: Is this relative to the start of the entry, or to the attribute?
	attrib_ofs: u32,
	indexed_flag: u8,
	_padding: u8,
	// name: [u16],

}
#[derive(::kernel_derives::FieldsLE)]
#[repr(C)]
pub struct MftAttrHeader_NonResident {
	/// First populated virtual cluster
	starting_vcn: u64,
	/// Last populated virtual cluster
	last_vcn: u64,

	data_run_ofs: u16,
	compression_unit_size: u16,
	_padding: u32,

	/// Total size of allocated clusters (in bytes)
	/// aka, Size on Disk
	allocated_size: u64,
	real_size: u64,
	/// One assumes, amount of actual data stored
	initiated_size: u64,
	// name: [u16],
}
