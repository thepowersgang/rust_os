// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/fs_fat/lib.rs
//! FAT (12/16/32) Filesystemd river
#![feature(linkage)]
#![no_std]

#[macro_use] extern crate kernel;
use kernel::prelude::*;

use kernel::metadevs::storage::{self,VolumeHandle,SizePrinter};
use kernel::lib::mem::aref::{ArefInner,ArefBorrow};
use kernel::lib::mem::Arc;
use ::vfs::{self, mount, node};

extern crate utf16;
extern crate block_cache;

module_define!{FS_FAT, [VFS], init}

const FAT16_MIN_CLUSTERS: usize = 4085;
const FAT32_MIN_CLUSTERS: usize = 65525;

/// FAT Legacy (pre 32) root cluster base. Just has to be above the max cluster num for FAT16
const FATL_ROOT_CLUSTER: u32 = 0x00FF0000;

/// Helper data-types
mod types;
/// on-disk structures
mod on_disk;
/// File-allocation-table management
mod fat;
/// Directory IO
mod dir;
/// File IO
mod file;

use types::{ClusterNum,ClusterList,InodeRef};

#[derive(Copy,Clone,Debug)]
enum Size
{
	Fat12,
	Fat16,
	Fat32,
}

/// Driver strucutre
struct Driver;

struct Filesystem
{
	inner: ArefInner<FilesystemInner>
}
impl ::core::ops::Deref for Filesystem {
	type Target = FilesystemInner;
	fn deref(&self) -> &FilesystemInner { &self.inner }
}

pub struct FilesystemInner
{
	//vh: VolumeHandle,
	vh: ::block_cache::CachedVolume,
	ty: Size,
	
	spc: usize,
	cluster_size: usize,
	/// Total number of data clusters
	cluster_count: usize,
	first_fat_sector: usize,
	first_data_sector: usize,
	
	root_first_cluster: ClusterNum,
	root_sector_count: u32,

	// TODO: Directory handles (with the dir's lock, and the number of open handles/files)
	dir_info: ::kernel::sync::RwLock<::kernel::lib::collections::VecMap<ClusterNum,Arc<dir::DirInfo>>>,
	open_files: ::kernel::sync::RwLock<::kernel::lib::collections::VecMap<ClusterNum,dir::OpenFileInfo>>,
}


static S_DRIVER: Driver = Driver;

fn init()
{
	let h = mount::DriverRegistration::new("fat", &S_DRIVER);
	// TODO: Remember the registration for unloading
	::core::mem::forget(h);
}

impl mount::Driver for Driver
{
	fn detect(&self, vol: &VolumeHandle) -> ::vfs::Result<usize> {
		let bs = {
			let mut bs = [0u8; 512];
			::kernel::futures::block_on( vol.read_blocks(0, &mut bs) )?;
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
	fn mount(&self, vol: VolumeHandle, _mounthandle: mount::SelfHandle) -> ::vfs::Result<Box<dyn mount::Filesystem>> {
		let vol = ::block_cache::CachedVolume::new(vol);

		// Read the bootsector
		let bs = {
			let blk = ::kernel::futures::block_on(vol.get_block(0))?;
			on_disk::BootSect::read(&mut &blk.data()[..512])
			};
		let bs_c = bs.common();
		if bs_c.bps != 512 {
			return Err(vfs::Error::Unknown("TODO: non 512-byte sector FAT"))
		}
		if bs_c.fat_count == 0 {
			return Err(vfs::Error::Unknown("FAT Count is 0"));
		}
		
		log_debug!("Label: {:?}", ::kernel::lib::RawString(&bs.tail_common().label));
		
		let bps = bs_c.bps as usize;
		let spc = bs_c.spc as usize;
		
		let root_dir_sectors = (bs_c.files_in_root as usize*32 + bps - 1) / bps;
		let fat_size = if bs_c.fat_size_16 > 0 {
				bs_c.fat_size_16 as usize
			}
			else {
				bs.info32().expect("Zero FAT size, and no 32 info").fat_size_32 as usize
			};
		let total_sectors = if bs_c.total_sectors_16 > 0 {
				bs_c.total_sectors_16 as usize
			}
			else {
				bs_c.total_sectors_32 as usize
			};
		
		// Calcualte some quantities
		let spare_fat_sectors = (bs_c.fat_count as usize - 1) * fat_size;
		let first_data_sector = bs_c.reserved_sect_count as usize
			+ fat_size + spare_fat_sectors
			+ root_dir_sectors;
		let cluster_count = (total_sectors - first_data_sector - spare_fat_sectors) / spc;
		
		// Determine the FAT type
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
				ty: fat_type,
				spc: spc,
				cluster_size: spc * vol.block_size(),
				cluster_count: cluster_count,
				first_fat_sector: bs_c.reserved_sect_count as usize,
				first_data_sector: first_data_sector,
				root_first_cluster: match fat_type {
					Size::Fat32 => ClusterNum::new(bs.info32().unwrap().root_cluster)
						.map_err(|()| {
							log_error!("Invalid FAT32 bootsector: bad root_cluster");
							vfs::Error::InconsistentFilesystem
							})?,
					_ => ClusterNum::new(FATL_ROOT_CLUSTER as u32).unwrap(),
					},
				root_sector_count: root_dir_sectors as u32,
				dir_info: Default::default(),
				open_files: Default::default(),

				vh: vol,
				}) },
			}))
	}
}

impl FilesystemInner
{
	fn get_sector_for_cluster(&self, cluster: ClusterNum) -> u64 {
		let cluster = cluster.get();
		if !is!(self.ty, Size::Fat32) && cluster >= FATL_ROOT_CLUSTER {
			// Root directory (for FAT12/16, where it was not a normal file)
			let rc = cluster - FATL_ROOT_CLUSTER;
			assert!( (rc as u64 * self.spc as u64) < self.root_sector_count as u64);
			(self.first_data_sector - self.root_sector_count as usize) as u64
			+ (rc * self.spc as u32) as u64
		}
		else {
			// Anything else
			assert!(cluster >= 2);
			assert!(cluster - 2 < self.cluster_count as u32);
			self.first_data_sector as u64 + (cluster as u64 - 2) * self.spc as u64
		}
	}
	/// Load a cluster from disk
	async fn read_clusters(&self, cluster: ClusterNum, dst: &mut [u8]) -> Result<(), storage::IoError> {
		log_trace!("Filesystem::read_clusters({}, {})", cluster, dst.len() / self.cluster_size);
		assert_eq!(dst.len() % self.cluster_size, 0);
		// For now, just read the bytes, screw caching
		let sector = self.get_sector_for_cluster(cluster);
		log_debug!("read_clusters: cluster = {}, sector = 0x{:x}", cluster, sector);
		self.vh.read_blocks(sector, dst).await?;
		//::kernel::logging::hex_dump("FAT Cluster", &buf);
		Ok( () )
	}
	/// Write to a cluster
	async fn write_clusters(&self, cluster: ClusterNum, src: &[u8]) -> Result<(), storage::IoError> {
		log_trace!("Filesystem::write_clusters({}, {})", cluster, src.len() / self.cluster_size);
		assert_eq!(src.len() % self.cluster_size, 0);
		// For now, just read the bytes, screw caching
		let sector = self.get_sector_for_cluster(cluster);
		log_debug!("write_clusters: cluster = {}, sector = 0x{:x}", cluster, sector);
		self.vh.write_blocks(sector, src).await?;
		Ok( () )
	}

	/// Cached cluster access
	async fn with_cluster<T>(&self, cluster: ClusterNum, callback: impl FnOnce(&[u8])->T) -> Result<T, storage::IoError> {
		let sector = self.get_sector_for_cluster(cluster);
		let block = self.vh.get_block(sector).await?;
		let ofs = sector - block.index();
		Ok( callback(&block.data()[ofs as usize * self.vh.block_size()..]) )
	}
	async fn edit_cluster(&self, cluster: ClusterNum, callback: impl FnOnce(&mut [u8])) -> Result<(), storage::IoError> {
		let sector = self.get_sector_for_cluster(cluster);
		// TODO: What if a cluster is larger than a block?
		self.vh.edit(sector, /*::block_cache::CacheType::Metadata,*/ self.spc, callback).await
	}
}

impl mount::Filesystem for Filesystem
{
	fn root_inode(&self) -> node::InodeId {
		InodeRef::root(self.root_first_cluster).to_id()
	}
	fn get_node_by_inode(&self, id: node::InodeId) -> Option<node::Node> {
		let r = InodeRef::from(id);
		if let Some(dir) = r.dir_first_cluster {
			// Reading from the directory starting at r.dir_first_cluster
			// locate the file with cluster equal to r.first_cluster.
			// And use that to create the node
			let dn = dir::DirNode::new(self.inner.borrow(),dir);
			dn.find_node(r.first_cluster).expect("TODO: Error for `get_node_by_inode`")
		}
		else {
			assert!(r.first_cluster == self.root_first_cluster);
			Some(node::Node::Dir(dir::DirNode::new_boxed(self.inner.borrow(), r.first_cluster)))
		}
	}
}
