// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/fs_fat/lib.rs
//! FAT (12/16/32) Filesystemd river
#![feature(no_std,core,linkage)]
#![no_std]
#[macro_use] extern crate core;
#[macro_use] extern crate kernel;
use kernel::prelude::*;

use kernel::vfs::{self, mount, node};
use kernel::metadevs::storage::{self,VolumeHandle,SizePrinter};
use kernel::lib::mem::aref::{ArefInner,ArefBorrow};

module_define!{FS_FAT, [VFS], init}

const FAT16_MIN_CLUSTERS: usize = 4085;
const FAT32_MIN_CLUSTERS: usize = 65525;

const FAT12_EOC: u16 = 0x0FFF;
const FAT16_EOC: u16 = 0xFFFF;
const FAT32_EOC: u32 = 0x00FFFFFF;

/// on-disk structures
mod on_disk;
/// Directory IO
mod dir;

#[derive(Copy,Clone,Debug)]
enum Size
{
	Fat12,
	Fat16,
	Fat32,
}

struct Driver;

struct Filesystem
{
	inner: ArefInner<FilesystemInner>
}
impl ::core::ops::Deref for Filesystem {
	type Target = FilesystemInner;
	fn deref(&self) -> &FilesystemInner { &self.inner }
}
struct FilesystemInner
{
	vh: VolumeHandle,
	ty: Size,
	
	spc: usize,
	cluster_count: usize,
	first_data_sector: usize,
	
	root_first_cluster: u32,
}

/// Inodes IDs destrucure into two 28-bit cluster IDs, and a 16-bit dir offset
#[derive(Debug)]
struct InodeRef
{
	dir_first_cluster: u32,
	dir_offset: u16,
	first_cluster: u32,
}

static S_DRIVER: Driver = Driver;

fn init()
{
	let h = mount::DriverRegistration::new("fat", &S_DRIVER);
	unsafe { ::core::mem::forget(h); }
}

impl mount::Driver for Driver
{
	fn detect(&self, vol: &VolumeHandle) -> vfs::Result<usize> {
		let bs = {
			let mut bs = [0u8; 512];
			try!( vol.read_blocks(0, &mut bs) );
			on_disk::BootSect::read(&bs)
			};
		
		let bps = bs.common().bps;
		let spc = bs.common().spc;
		let media_desc = bs.common().media_descriptor;
		
		if bps == 0 || spc == 0 || media_desc < 0xf0 {
			Ok(0)
		}
		else {
			Ok(1)
		}
	}
	fn mount(&self, vol: VolumeHandle) -> vfs::Result<Box<mount::Filesystem>> {
		let bs = {
			let mut bs = [0u8; 512];
			try!( vol.read_blocks(0, &mut bs) );
			on_disk::BootSect::read(&mut &bs[..])
			};
		let bs_c = bs.common();
		if bs_c.bps != 512 {
			return Err(vfs::Error::Unknown("TODO: non 512-byte sector FAT"))
		}
		
		let bps = bs_c.bps as usize;
		let spc = bs_c.spc as usize;
		
		let root_dir_sectors = (bs_c.files_in_root as usize*32 + bps - 1) / bps;
		let fat_size = if bs_c.fat_size_16 > 0 {
				bs_c.fat_size_16 as usize
			}
			else {
				todo!("FAT32 FAT size field");
			};
		let total_sectors = if bs_c.total_sectors_16 > 0 {
				bs_c.total_sectors_16 as usize
			}
			else {
				bs_c.total_sectors_32 as usize
			};
		
		let fat_sectors = bs_c.fat_count as usize * fat_size;
		let non_data_sectors = bs_c.reserved_sect_count as usize
			+ fat_sectors
			+ root_dir_sectors;
		let cluster_count = (total_sectors - non_data_sectors) / spc;
		
		let fat_type = if cluster_count < FAT16_MIN_CLUSTERS {
				Size::Fat12
			}
			else if cluster_count < FAT32_MIN_CLUSTERS {
				Size::Fat16
			}
			else {
				Size::Fat32
			};
		log_debug!("{:?} {} sectors, Size {}", fat_type, total_sectors,
			SizePrinter((total_sectors*bs_c.bps as usize) as u64));
		
		Ok(Box::new(Filesystem {
			// SAFE: Saving to a Box, so won't move
			inner: unsafe { ArefInner::new(FilesystemInner {
				vh: vol,
				ty: fat_type,
				spc: spc,
				cluster_count: cluster_count,
				first_data_sector: non_data_sectors,
				root_first_cluster: match fat_type {
					Size::Fat32 => bs.info32().unwrap().root_cluster,
					_ => (fat_sectors / spc) as u32,
					},
				}) },
			}))
	}
}


impl mount::Filesystem for Filesystem
{
	fn root_inode(&self) -> node::InodeId {
		(InodeRef {
			first_cluster: self.root_first_cluster,
			dir_first_cluster: 0,
			dir_offset: 0,
			}).to_id()
	}
	fn get_node_by_inode(&self, id: node::InodeId) -> Option<node::Node> {
		let r = InodeRef::from(id);
		if r.first_cluster == self.root_first_cluster {
			if let Size::Fat32 = self.ty {
				Some(node::Node::Dir(Box::new(dir::DirNode::new(self, r.first_cluster))))
			}
			else {
				Some(node::Node::Dir(Box::new(dir::RootDirNode::new(self))))
			}
		}
		else {
			// Reading from the directory starting at r.dir_first_cluster
			// locate the file with cluster equal to r.first_cluster.
			// And use that to create the node
			todo!("get_node_by_inode - r = {:?}", r);
		}
	}
}

impl InodeRef
{
	fn to_id(&self) -> node::InodeId {
		assert!(self.first_cluster <= 0x00FF_FFFF);
		assert!(self.dir_first_cluster <= 0x00FF_FFFF);
		//assert!(v.dir_offset <= 0xFFFF);
		(self.first_cluster as u64)
		| (self.dir_first_cluster as u64) << 24
		| (self.dir_offset as u64) << 48
	}
}

impl From<node::InodeId> for InodeRef {
	fn from(v: node::InodeId) -> InodeRef {
		InodeRef {
			first_cluster: (v & 0x00FF_FFFF) as u32,
			dir_first_cluster: ((v >> 24) & 0x00FF_FFFF) as u32,
			dir_offset: (v >> 48) as u16,
		}
	}
}
