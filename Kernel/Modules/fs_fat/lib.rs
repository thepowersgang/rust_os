// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/fs_fat/lib.rs
//! FAT (12/16/32) Filesystemd river
#![feature(linkage)]
#![no_std]

#[macro_use] extern crate kernel;
use kernel::prelude::*;

use kernel::vfs::{self, mount, node};
use kernel::metadevs::storage::{self,VolumeHandle,SizePrinter};
use kernel::lib::mem::aref::{ArefInner,ArefBorrow};
use kernel::lib::mem::Arc;

extern crate utf16;
extern crate blockcache;
extern crate block_cache;

module_define!{FS_FAT, [VFS], init}

const FAT16_MIN_CLUSTERS: usize = 4085;
const FAT32_MIN_CLUSTERS: usize = 65525;

/// FAT Legacy (pre 32) root cluster base. Just has to be above the max cluster num for FAT16
const FATL_ROOT_CLUSTER: u32 = 0x00FF0000;

const FAT12_EOC: u16 = 0x0FFF;
const FAT16_EOC: u16 = 0xFFFF;
const FAT32_EOC: u32 = 0x00FFFFFF;

/// on-disk structures
mod on_disk;
/// Directory IO
mod dir;
/// File IO
mod file;

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

//const FAT_CACHE_BLOCK_SIZE: usize = 512;
pub struct FilesystemInner
{
	//vh: VolumeHandle,
	vh: ::block_cache::CacheHandle,
	ty: Size,
	
	spc: usize,
	cluster_size: usize,
	/// Total number of data clusters
	cluster_count: usize,
	first_fat_sector: usize,
	first_data_sector: usize,
	
	root_first_cluster: u32,
	root_sector_count: u32,
	
	//fat_cache: vfs::Cache<[u32; FAT_CACHE_BLOCK_SIZE]>,
	// XXX: Should really use the above line for this, but BlockCache exists
	/// A cache of metadata clusters (i.e. directories)
	metadata_block_cache: ::blockcache::BlockCache,
}

/// Inodes IDs destrucure into two 28-bit cluster IDs, and a 16-bit dir offset
#[derive(Debug)]
struct InodeRef
{
	dir_first_cluster: u32,
	dir_offset: u16,
	first_cluster: u32,
}

/// Iterable cluster list
enum ClusterList {
	Range(::core::ops::Range<u32>),
	Chained(ArefBorrow<FilesystemInner>, u32),
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
	fn detect(&self, vol: &VolumeHandle) -> vfs::Result<usize> {
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
	fn mount(&self, vol: VolumeHandle, _mounthandle: mount::SelfHandle) -> vfs::Result<Box<dyn mount::Filesystem>> {
		let vol = ::block_cache::CacheHandle::new(vol);

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
					Size::Fat32 => bs.info32().unwrap().root_cluster,
					_ => FATL_ROOT_CLUSTER as u32,
					},
				root_sector_count: root_dir_sectors as u32,
				
				metadata_block_cache: ::blockcache::BlockCache::new(),

				vh: vol,
				}) },
			}))
	}
}

type Cluster = Arc<[u8]>;

impl FilesystemInner
{
	/// Load a cluster from disk
	async fn read_cluster(&self, cluster: u32, dst: &mut [u8]) -> Result<(), storage::IoError> {
		assert_eq!(dst.len(), self.cluster_size);
		self.read_clusters(cluster, dst).await
	}
	async fn read_clusters(&self, cluster: u32, dst: &mut [u8]) -> Result<(), storage::IoError> {
		log_trace!("Filesystem::read_clusters({:#x}, {})", cluster, dst.len() / self.cluster_size);
		assert_eq!(dst.len() % self.cluster_size, 0);
		// For now, just read the bytes, screw caching
		let sector = if !is!(self.ty, Size::Fat32) && cluster >= FATL_ROOT_CLUSTER {
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
			};
		log_debug!("read_clusters: cluster = {:#x}, sector = 0x{:x}", cluster, sector);
		self.vh.read_blocks(sector, dst).await?;
		//::kernel::logging::hex_dump("FAT Cluster", &buf);
		Ok( () )
	}

	// TODO: Locking/Cache
	// - Should this function lock the cluster somehow to prevent accidental overlap?
	// - Could also cache somehow (with a refcount) along with the 'writing' flag
	fn load_cluster(&self, cluster: u32) -> Result<Cluster, storage::IoError>
	{
		self.metadata_block_cache.get(
			cluster,
			|_| {
				log_debug!("load_cluster: miss {}", cluster);
				let mut buf: Cluster = Arc::from_iter( (0..self.spc * self.vh.block_size()).map(|_| 0) );
				::kernel::futures::block_on( self.read_cluster( cluster, Arc::get_mut(&mut buf).unwrap() ) )?;
				Ok( buf )
			})
	}
	
	/// Obtain the next cluster in a chain
	fn get_next_cluster(&self, cluster: u32) -> Result< Option<u32>, storage::IoError > {
		use kernel::lib::byteorder::{ReadBytesExt,LittleEndian};
		// - Determine what sector contains the requested FAT entry
		let bs = self.vh.block_size();
		let (fat_sector,ofs) = match self.ty
			{
			Size::Fat12 => {
				let cps = bs / 3 * 2;	// 2 per 3 bytes
				(cluster as usize / cps, (cluster as usize % cps) / 2 * 3 )
				},
			Size::Fat16 => {
				let cps = bs / 2;
				(cluster as usize / cps, (cluster as usize % cps) * 2)
				},
			Size::Fat32 => {
				let cps = bs / 4;
				(cluster as usize / cps, (cluster as usize % cps) * 2)
				},
			};

		// - Read a single sector from the FAT
		let sector_idx = (self.first_fat_sector + fat_sector) as u64;
		let sector_data_blk = ::kernel::futures::block_on(self.vh.get_block( sector_idx ))?;
		let start_ofs = (sector_idx - sector_data_blk.index()) as usize * bs;
		let sector_data = &sector_data_blk.data()[start_ofs .. ];
		//log_debug!("Sector {} accessed via cached block at sector {}", sector_idx, sector_data_blk.index());
		//::kernel::logging::hex_dump("FAT FAT", &sector_data[..bs]);

		// - Extract the entry
		Ok(match self.ty
		{
		Size::Fat12 => {
			// FAT12 has special handling because it packs 2 entries into 24 bytes
			let val = {
				let v24 = (&sector_data[ofs..]).read_uint::<LittleEndian>(3).unwrap();
				if cluster % 2 == 0 { v24 & 0xFFF } else { v24 >> 12 }
				} as u16;
			if val == 0 { return Err(storage::IoError::Unknown("FAT: Zero FAT entry")); }
			if val == FAT12_EOC { None } else { Some(val as u32) }
			},
		Size::Fat16 => {
			let val = (&sector_data[ofs..]).read_u16::<LittleEndian>().unwrap();
			if val == 0 { return Err(storage::IoError::Unknown("FAT: Zero FAT entry")); }
			if val == FAT16_EOC { None } else { Some(val as u32) }
			},
		Size::Fat32 => {
			let val = (&sector_data[ofs..]).read_u32::<LittleEndian>().unwrap();
			if val == 0 { return Err(storage::IoError::Unknown("FAT: Zero FAT entry")); }
			if val == FAT32_EOC { None } else { Some(val as u32) }
			},
		})
	}

	fn alloc_cluster(&self, prev_cluster: u32) -> Result< u32, storage::IoError > {
		todo!("alloc_cluster(prev={:#x})", prev_cluster)
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
			Some(node::Node::Dir(dir::DirNode::new_boxed(self.inner.borrow(), r.first_cluster)))
		}
		else {
			// Reading from the directory starting at r.dir_first_cluster
			// locate the file with cluster equal to r.first_cluster.
			// And use that to create the node
			let dn = dir::DirNode::new(self.inner.borrow(), r.dir_first_cluster);
			dn.find_node(r.first_cluster)
		}
	}
}

impl InodeRef
{
	fn new(c: u32, dir_c: u32) -> InodeRef {
		assert!(c     <= 0x00FF_FFFF);
		assert!(dir_c <= 0x00FF_FFFF);
		InodeRef {
			first_cluster: c,
			dir_first_cluster: dir_c,
			dir_offset: 0,
		}
	}
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

impl ClusterList {
	pub fn chained(fs: ArefBorrow<FilesystemInner>, start: u32) -> ClusterList {
		ClusterList::Chained(fs, start)
	}

	/// Returns an extent of at most `max_clusters` contigious clusters
	pub fn next_extent(&mut self, max_clusters: usize) -> Option<(u32, usize)> {
		match *self
		{
		ClusterList::Range(ref mut r) => r.next().map(|v| (v, 1)),
		ClusterList::Chained(ref fs, ref mut next) =>
			if *next == 0 {
				None
			}
			else {
				let rv = *next;
				let mut count = 0;
				while *next != 0 && *next == rv + count as u32 && count < max_clusters
				{
					*next = match fs.get_next_cluster(*next)
						{
						Ok(Some(v)) => v,
						Ok(None) => 0,
						Err(e) => {
							log_warning!("Error when reading cluster chain - {:?}", e);
							return None;	// Inconsistency, terminate asap
							},
						};
					count += 1;
				}
				Some( (rv, count) )
			},
		}
	}
}
impl ::core::iter::Iterator for ClusterList {
	type Item = u32;
	fn next(&mut self) -> Option<u32> {
		match *self
		{
		ClusterList::Range(ref mut r) => r.next(),
		ClusterList::Chained(ref fs, ref mut next) =>
			if *next == 0 {
				None
			}
			else {
				let rv = *next;
				*next = match fs.get_next_cluster(*next)
					{
					Ok(Some(v)) => v,
					Ok(None) => 0,
					Err(e) => {
						log_warning!("Error when reading cluster chain - {:?}", e);
						return None;	// Inconsistency, terminate asap
						},
					};
				Some( rv )
			},
		}
	}
}
