// "Tifflin" Kernel - FAT Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_fat/on_disk.rs
//! On-Disk structures and flags
#[allow(unused_imports)]
use kernel::prelude::*;

pub const ATTR_READONLY : u8 = 0x01;	// Read-only file
pub const ATTR_HIDDEN   : u8 = 0x02;	// Hidden File
pub const ATTR_SYSTEM   : u8 = 0x04;	// System File
pub const ATTR_VOLUMEID : u8 = 0x08;	// Volume ID (Deprecated)
pub const ATTR_DIRECTORY: u8 = 0x10;	// Directory
pub const ATTR_LFN: u8 = ATTR_READONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_VOLUMEID;
#[allow(dead_code)]
pub const ATTR_ARCHIVE  : u8 = 0x20;	// Flag set by user

pub const CASE_LOWER_BASE: u8 = 0x08;	// Linux (maybe NT) flag
pub const CASE_LOWER_EXT : u8 = 0x10;	// Linux (maybe NT) flag

fn read_u8(s: &mut &[u8]) -> u8 {
	use kernel::lib::byteorder::ReadBytesExt;
	s.read_u8().unwrap()
}
fn read_u16(s: &mut &[u8]) -> u16 {
	use kernel::lib::byteorder::{ReadBytesExt,LittleEndian};
	s.read_u16::<LittleEndian>().unwrap()
}
fn read_u32(s: &mut &[u8]) -> u32 {
	use kernel::lib::byteorder::{ReadBytesExt,LittleEndian};
	s.read_u32::<LittleEndian>().unwrap()
}
fn read_arr<T: AsMut<[u8]>>(s: &mut &[u8]) -> T {
	use kernel::lib::io::Read;
	// (mostly) SAFE: 'T' should be POD... but can't enforce that easily
	let mut v: T = unsafe { ::core::mem::zeroed() };
	s.read(v.as_mut()).unwrap();
	v
}
fn read_arr16<T: AsMut<[u16]>>(s: &mut &[u8]) -> T {
	// (mostly) SAFE: 'T' should be POD... but can't enforce that easily
	let mut v: T = unsafe { ::core::mem::zeroed() };
	for p in v.as_mut() {
		*p = read_u16(s);
	}
	v
}

pub enum BootSect
{
	Legacy(BootSect16),
	Fat32(BootSect32),
}
impl BootSect {
	pub fn read(src: &[u8]) -> BootSect {
		assert_eq!(src.len(), 512);
		let mut s = src;
		let common = BootSectInfo::read(&mut s);
		assert_eq!(s.len(), 512-0x24);
		if common.fat_size_16 > 0 {
			let rv = BootSect::Legacy( BootSect16 {
				common: common,
				info: BootSect16Info::read(&mut s),
				});
			assert_eq!(s.len(), 512-(0x24+26));
			rv
		}
		else {
			let rv = BootSect::Fat32( BootSect32 {
				common: common,
				info32: BootSect32Info::read(&mut s),
				info16: BootSect16Info::read(&mut s),
				});
			assert_eq!(s.len(), 512-90);
			rv
		}
	}
	pub fn common(&self) -> &BootSectInfo {
		match self
		{
		&BootSect::Legacy(ref i) => &i.common,
		&BootSect::Fat32(ref i) => &i.common,
		}
	}
	pub fn info32(&self) -> Option<&BootSect32Info> {
		match self
		{
		&BootSect::Legacy(_) => None,
		&BootSect::Fat32(ref i) => Some(&i.info32),
		}
	}
	pub fn tail_common(&self) -> &BootSect16Info {
		match self
		{
		&BootSect::Legacy(ref i) => &i.info,
		&BootSect::Fat32(ref i) => &i.info16,
		}
	}
}

pub struct BootSectInfo
{
	pub _jump: [u8; 3],
	pub oem_name: [u8; 8],
	pub bps: u16,
	pub spc: u8,
	pub reserved_sect_count: u16,
	pub fat_count: u8,
	pub files_in_root: u16,
	pub total_sectors_16: u16,
	pub media_descriptor: u8,
	pub fat_size_16: u16,
	
	pub _spt: u16,
	pub _heads: u16,
	pub _hidden_count: u32,
	pub total_sectors_32: u32,
}
impl BootSectInfo {
	fn read(src: &mut &[u8]) -> BootSectInfo {
		BootSectInfo {
			_jump: read_arr(src),
			oem_name: read_arr(src),
			bps: read_u16(src),
			spc: read_u8(src),
			reserved_sect_count: read_u16(src),
			fat_count: read_u8(src),
			files_in_root: read_u16(src),
			total_sectors_16: read_u16(src),
			media_descriptor: read_u8(src),
			fat_size_16: read_u16(src),

			_spt: read_u16(src),
			_heads: read_u16(src),
			_hidden_count: read_u32(src),
			total_sectors_32: read_u32(src),
		}
	}
}
pub struct BootSect16
{
	common: BootSectInfo,
	info: BootSect16Info,
}
pub struct BootSect16Info
{
	pub drive_num: u8,
	_rsvd: u8,
	pub boot_sig: u8,
	pub vol_id: u32,
	pub label: [u8; 11],
	pub fs_type: [u8; 8],
}
impl BootSect16Info {
	fn read(src: &mut &[u8]) -> BootSect16Info {
		BootSect16Info {
			drive_num: read_u8(src),
			_rsvd:	 read_u8(src),
			boot_sig:  read_u8(src),
			vol_id:	read_u32(src),
			label:   read_arr(src),
			fs_type: read_arr(src),
		}
	}
}
pub struct BootSect32
{
	common: BootSectInfo,
	info32: BootSect32Info,
	info16: BootSect16Info,
}
pub struct BootSect32Info
{
	pub fat_size_32: u32,
	pub ext_flags: u16,
	pub fs_ver: u16,
	pub root_cluster: u32,
	pub fs_info: u16,
	pub backup_bootsect: u16,
	_resvd: [u8; 12],
}
impl BootSect32Info {
	fn read(src: &mut &[u8]) -> BootSect32Info {
		BootSect32Info {
			fat_size_32: read_u32(src),
			ext_flags: read_u16(src),
			fs_ver: read_u16(src),
			root_cluster: read_u32(src),
			fs_info: read_u16(src),
			backup_bootsect: read_u16(src),
			_resvd: read_arr(src),
		}
	}
}

#[derive(Debug)]
pub struct DirEnt
{
	pub name: [u8; 11],
	pub attribs: u8,
	pub lcase: u8,
	pub creation_ds: u8,	// 10ths of a second
	pub creation_time: u16,
	pub creation_date: u16,
	pub accessed_date: u16,
	pub cluster_hi: u16,
	pub modified_time: u16,
	pub modified_date: u16,
	pub cluster: u16,
	pub size: u32,
}
impl DirEnt {
	pub fn read(src: &mut &[u8]) -> DirEnt {
		DirEnt {
			name: read_arr(src),
			attribs: read_u8(src),
			lcase: read_u8(src),
			creation_ds: read_u8(src),
			creation_time: read_u16(src),
			creation_date: read_u16(src),
			accessed_date: read_u16(src),
			cluster_hi: read_u16(src),
			modified_time: read_u16(src),
			modified_date: read_u16(src),
			cluster: read_u16(src),
			size: read_u32(src),
		}
	}
}
#[derive(Debug)]
pub struct DirEntLong
{
	pub id: u8,
	pub name1: [u16; 5],
	pub attrib: u8,	// Must be ATTR_LFN
	pub ty: u8,	// Dunno?
	pub checksum: u8,
	pub name2: [u16; 6],
	pub first_cluster: u16,
	pub name3: [u16; 2],
}
impl DirEntLong {
	pub fn read(src: &mut &[u8]) -> DirEntLong {
		DirEntLong {
			id: read_u8(src),
			name1: read_arr16(src),
			attrib: read_u8(src),
			ty: read_u8(src),
			checksum: read_u8(src),
			name2: read_arr16(src),
			first_cluster: read_u16(src),
			name3: read_arr16(src),
		}
	}
}

