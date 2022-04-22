// "Tifflin" Kernel - ISO9660 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_iso9660/lib.rs
#![feature(linkage)]
#![no_std]
use kernel::prelude::*;

use kernel::vfs::{self, mount, node};
use kernel::metadevs::storage::{self,VolumeHandle};
use kernel::lib::mem::aref::{ArefInner,ArefBorrow};
use kernel::lib::byteorder::{ByteOrder,LittleEndian};
use kernel::lib::byte_str::ByteStr;

#[macro_use]
extern crate kernel;

extern crate block_cache;

module_define!{FS_ISO9660, [VFS], init}

//mod ondisk;

struct Driver;
static S_DRIVER: Driver = Driver;

struct Instance(ArefInner<InstanceInner>);
impl ::core::ops::Deref for Instance {
	type Target = InstanceInner;
	fn deref(&self) -> &InstanceInner { &self.0 }
}

struct InstanceInner
{
	vh: ::block_cache::CacheHandle,
	lb_size: usize,
	root_lba: u32,
	root_size: u32,

	susp_len_skip: Option<u8>,
}

fn init()
{
	let h = mount::DriverRegistration::new("iso9660", &S_DRIVER);
	// TODO: Remember the registration for unloading?
	::core::mem::forget(h);
}

impl mount::Driver for Driver
{
	fn detect(&self, vol: &VolumeHandle) -> vfs::Result<usize> {
		let bs = vol.block_size() as u64;
		let blk = {
			let mut block: Vec<_> = (0 .. bs).map(|_|0).collect();
			::kernel::futures::block_on(vol.read_blocks(32*1024 / bs, &mut block))?;
			block
			};
		if &blk[1..6] == b"CD001" {
			Ok(1)
		}
		else {
			Ok(0)
		}
	}
	fn mount(&self, vol: VolumeHandle, _mounthandle: mount::SelfHandle) -> vfs::Result<Box<dyn mount::Filesystem>> {
		// For this to work properly, the block size must evenly divide 2048
		if 2048 % vol.block_size() != 0 {
			return Err( vfs::Error::Unknown("Can't mount ISO9660 with sector size not a factor of 2048"/*, vol.block_size()*/) );
		}
		let scale = 2048 / vol.block_size();
		
		// Search the start of the disk for the primary volume descriptor
		// - TODO: Limit the number of sectors searched.
		let mut block = vec![0u8; 2048];
		for sector in 16 .. 
		{
			::kernel::futures::block_on(vol.read_blocks((sector*scale) as u64, &mut block))?;
			if &block[1..6] != b"CD001" {
				return Err( vfs::Error::Unknown("Invalid volume descriptor present") );
			}
			else if block[0] == 255 {
				return Err( vfs::Error::Unknown("Can't find ISO9660 primary volume descriptor") );
			}
			else if block[0] == 0x01 {
				// Found it!
				break ;
			}
			else {
				// Try the next one
			}
		}
		//::kernel::logging::hex_dump("ISO966 PVD", &block);
		
		// Obtain the logical block size (different from medium sector size)
		let lb_size = LittleEndian::read_u16(&block[128..]);
		// Extract the root directory entry
		// - We want the LBA and byte length
		let root_lba  = LittleEndian::read_u32(&block[156+ 2..]);
		let root_size = LittleEndian::read_u32(&block[156+10..]);
		
		log_debug!("lb_size = {}, root = {:#x} + {:#x} bytes", lb_size, root_lba, root_size);
	
	
		let mut inner = InstanceInner {
			vh: ::block_cache::CacheHandle::new(vol),
			lb_size: lb_size as usize,
			root_lba: root_lba,
			root_size: root_size,
			susp_len_skip: None,
			};

		// Determine if SUSP is in use (used for RockRidge extensions)
		inner.susp_len_skip = {
			let mut it = DirSector::new(&inner, ::kernel::futures::block_on(inner.get_sector(root_lba))?, 0 );
			let first_ent = match it.next()?
				{
				None => return Err(vfs::Error::InconsistentFilesystem),
				Some(v) => v,
				};
			if first_ent.sys_use.len() >= 6 && &first_ent.sys_use[..6] == b"SP\x07\x01\xBE\xEF" {
				Some(first_ent.sys_use[6])
			}
			else {
				None
			}
			};
		
		// SAFE: Stored in a box, and not moved out.
		Ok( Box::new( Instance(unsafe { ArefInner::new( inner ) }) ) )
	}
}

impl mount::Filesystem for Instance
{
	fn root_inode(&self) -> node::InodeId {
		0 as node::InodeId
	}
	fn get_node_by_inode(&self, id: node::InodeId) -> Option<node::Node> {
		if id == 0 {
			Some(Dir::new_node(self.0.borrow(), self.root_lba, self.root_size) )
		}
		else {
			// Look up (or read) parent directory to obtain the info
			let (sector, ofs) = ::kernel::lib::num::div_rem(id as u64, self.lb_size as u64);
			let blk = match ::kernel::futures::block_on(self.get_sector(sector as u32))
				{
				Ok(v) => v,
				Err(_) => return None,
				};
			let mut it = DirSector::new(&self.0, blk, ofs as usize);
			let ent = match it.next()
				{
				Ok(Some(v)) => v,
				Ok(None) => return None,
				Err(_) => return None,
				};
			if ent.name.len() == 0 {
				None
			}
			else {
				if ent.flags & (1 << 7) != 0 {
					// Multi-extent file!
					None
				}
				else if ent.flags & (1 << 1) != 0 {
					Some(Dir::new_node(self.0.borrow(), ent.start, ent.size))
				}
				else if ent.flags & 0x64 != 0 {
					None
				}
				else {
					Some(File::new_node(self.0.borrow(), ent.start, ent.size))
				}
			}
		}
	}
}
struct Sector<'a>(::block_cache::CachedBlockHandle<'a>,u16,u16);
impl<'a> ::core::ops::Deref for Sector<'a> {
	type Target = [u8];
	fn deref(&self) -> &[u8] {
		&self.0.data()[self.1 as usize ..][.. self.2 as usize]
	}
}
impl InstanceInner
{
	/// Read a sector from the disk into the provided buffer
	async fn read_sector(&self, sector: u32, buf: &mut [u8]) -> Result<(), storage::IoError> {
		assert_eq!(buf.len() % self.lb_size, 0);
		// - Will be round, Driver::mount() ensures this
		let hwsects_per_lb = self.lb_size / self.vh.block_size();
		let hwsector = sector as usize * hwsects_per_lb;
		self.vh.read_blocks( hwsector as u64, buf).await?;
		Ok( () )
		
	}
	/// Read a metadata sector via a cache
	async fn get_sector(&self, sector: u32) -> Result<Sector<'_>, storage::IoError> {
		assert!(sector > 0);
		
		let blk = self.vh.get_block(sector as u64).await?;
		let ofs = (sector as u64 - blk.index()) as usize * self.vh.block_size();
		Ok( Sector(blk, ofs as u16, self.vh.block_size() as u16) )
	}
}

// --------------------------------------------------------------------
struct File
{
	fs: ArefBorrow<InstanceInner>,
	first_lba: u32,
	size: u32,
}
impl File
{
	fn new_node(fs: ArefBorrow<InstanceInner>, first_lba: u32, size: u32) -> node::Node {
		node::Node::File( Box::new( File {
			fs: fs,
			first_lba: first_lba,
			size: size,
			} ) )
	}
}
impl node::NodeBase for File
{
	fn get_id(&self) -> node::InodeId {
		todo!("File::get_id")
	}
	fn get_any(&self) -> &dyn core::any::Any {
		self
	}
}
impl node::File for File
{
	fn size(&self) -> u64 {
		self.size as u64
	}
	fn truncate(&self, _newsize: u64) -> node::Result<u64> {
		Err(vfs::Error::ReadOnlyFilesystem)
	}
	fn clear(&self, _ofs: u64, _size: u64) -> node::Result<()> {
		Err(vfs::Error::ReadOnlyFilesystem)
	}
	fn read(&self, ofs: u64, buf: &mut [u8]) -> node::Result<usize> {
		if ofs > self.size as u64 {
			Err(vfs::Error::InvalidParameter)
		}
		else {
			let len = ::core::cmp::min( buf.len(),  (self.size as u64 - ofs) as usize );
			
			let (sector, ofs) = ::kernel::lib::num::div_rem(ofs, self.fs.lb_size as u64);
			let mut sector = sector as u32;
			let ofs = ofs as usize;

			let mut read = 0;
			// 1. Leading
			if ofs > 0 {
				log_trace!("ofs {} > 0, reading partial at {}", ofs, sector);
				let mut tmp = vec![0u8; self.fs.lb_size];
				::kernel::futures::block_on(self.fs.read_sector(self.first_lba + sector, &mut tmp))?;

				assert!(ofs < self.fs.lb_size);
				sector += 1;
				buf[..len].clone_from_slice(&tmp[ofs..]);
				read = len;
			}

			// 2. Inner
			if len - read >= self.fs.lb_size {
				let sector_count = (len - read) / self.fs.lb_size;
				log_trace!("reading {} sectors worth of data at {} (len = {}, read = {})", sector_count, sector, len, read);
				let bytes = sector_count * self.fs.lb_size;
				::kernel::futures::block_on(self.fs.read_sector(self.first_lba + sector, &mut buf[read..][..bytes]))?;
				sector += sector_count as u32;
				read += bytes;
			}

			// 3. Trailing
			if read < len {
				log_trace!("reading {} bytes trailing at {}", len - read, sector);
				let mut tmp = vec![0; self.fs.lb_size];
				::kernel::futures::block_on(self.fs.read_sector(self.first_lba + sector, &mut tmp))?;

				buf[read..].clone_from_slice(&tmp);
			}

			Ok( len )
		}
	}
	fn write(&self, _ofs: u64, _buf: &[u8]) -> node::Result<usize> {
		Err(vfs::Error::ReadOnlyFilesystem)
	}
}


// --------------------------------------------------------------------
struct Dir
{
	fs: ArefBorrow<InstanceInner>,
	first_lba: u32,
	size: u32,
}
impl Dir
{
	fn new_node(fs: ArefBorrow<InstanceInner>, first_lba: u32, size: u32) -> node::Node {
		node::Node::Dir( Box::new( Dir {
			fs: fs,
			first_lba: first_lba,
			size: size,
			} ) )
	}
}
impl node::NodeBase for Dir
{
	fn get_id(&self) -> node::InodeId {
		todo!("Dir::get_id")
	}
	fn get_any(&self) -> &dyn core::any::Any {
		self
	}
}
impl node::Dir for Dir
{
	fn lookup(&self, name: &ByteStr) -> node::Result<node::InodeId>
	{
		for sector in 0 .. ::kernel::lib::num::div_up(self.size, self.fs.lb_size as u32)
		{
			let mut it = DirSector::new(&self.fs, ::kernel::futures::block_on(self.fs.get_sector(self.first_lba + sector))?, 0); 

			while let Some(ent) = it.next()?
			{
				if ent.name == name.as_bytes()
				{
					let inode = (self.first_lba + sector) as u64 * self.fs.lb_size as u64 + ent.this_ofs as u64;
					return Ok( inode );
				}
			}
		}

		Err(vfs::Error::NotFound)
	}
	fn read(&self, ofs: usize, callback: &mut node::ReadDirCallback) -> node::Result<usize>
	{
		let max_sectors = ::kernel::lib::num::div_up(self.size, self.fs.lb_size as u32);
		let end_ofs = self.size as usize % self.fs.lb_size;

		let (sector, mut ofs) = (ofs / self.fs.lb_size, ofs % self.fs.lb_size);
		
		for sector in sector as u32 .. max_sectors
		{
			let mut it = DirSector::new(&self.fs, ::kernel::futures::block_on(self.fs.get_sector(self.first_lba + sector))?,  ofs );
			ofs = 0;

			while let Some(ent) = it.next()?
			{
				if ent.name.len() > 0 && ent.name != b"\0" && ent.name != b"\x01"
				{
					log_debug!("ent = {:?}", ent);
					let inode = (self.first_lba + sector) as u64 * self.fs.lb_size as u64 + ent.this_ofs as u64;
					if ! callback(inode, &mut ent.name.iter().cloned()) {
						return Ok( sector as usize * self.fs.lb_size + ent.next_ofs );
					}
				}
				if ent.next_ofs == end_ofs {
					break;
				}
			}
		}
		
		Ok( max_sectors as usize * self.fs.lb_size )
	}
	
	fn create(&self, _name: &ByteStr, _nodetype: node::NodeType) -> node::Result<node::InodeId> {
		// ISO9660 is readonly
		Err( vfs::Error::ReadOnlyFilesystem )
	}
	fn link(&self, _name: &ByteStr, _node: &dyn node::NodeBase) -> node::Result<()> {
		// ISO9660 is readonly
		Err( vfs::Error::ReadOnlyFilesystem )
	}
	fn unlink(&self, _name: &ByteStr) -> node::Result<()> {
		// ISO9660 is readonly
		Err( vfs::Error::ReadOnlyFilesystem )
	}
}


#[derive(Default)]
struct DirEnt<'a>
{
	this_ofs: usize,
	next_ofs: usize,

	flags: u8,
	start: u32,
	size: u32,
	name: &'a [u8],
	sys_use: &'a [u8],
}
impl<'a> ::core::fmt::Debug for DirEnt<'a> {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		write!(f, "DirEnt {{ start: {:#x}, size: {:#x}, name: {:?} }}",
			self.start, self.size, ByteStr::new(self.name)
			)
	}
}

impl<'a> DirEnt<'a>
{
}

struct DirSector<'a> {
	fs: &'a InstanceInner,
	data: Sector<'a>,
	ofs: usize
}

impl<'a> DirSector<'a>
{
	pub fn new<'b>(fs: &'b InstanceInner, data: Sector<'b>, start_ofs: usize) -> DirSector<'b>
	{
		DirSector {
			fs: fs,
			data: data,
			ofs: start_ofs,
		}
	}
	pub fn next(&mut self) -> node::Result<Option<DirEnt>> {
		let cur_ofs = self.ofs;

		if self.ofs == self.data.len() {
			Ok(None)
		}
		else {
			let len = self.data[self.ofs] as usize;
			if len == 0 {
				self.ofs += 1;
				Ok(Some(DirEnt {
					this_ofs: cur_ofs,
					next_ofs: self.ofs,
					..Default::default()
					}))
			}
			else if len < 33 {
				log_warning!("Consistency error in filesystem (dir entry length {} < 33)", len);
				return Err(vfs::Error::InconsistentFilesystem);
			}
			else if self.ofs + len > self.data.len() {
				log_warning!("Consistency error in filesystem (dir entry spans sectors)");
				return Err(vfs::Error::InconsistentFilesystem);
			}
			else {
				let ent = &self.data[self.ofs ..][.. len];
				self.ofs += len;
				
				let namelen = ent[32] as usize;
				if 33 + namelen > len {
					log_warning!("Name overruns end of entry");
					return Err(vfs::Error::InconsistentFilesystem);
				}
				let su = &ent[33 + namelen ..];

				let mut name = &ent[33..][..namelen];

				if let Some(skip) = self.fs.susp_len_skip {
					let skip = skip as usize;
					if su.len() < skip {
						log_warning!("System use area smaller than SUSP skip value");
						return Err(vfs::Error::InconsistentFilesystem);
					}

					for ent in SuspIterator(&su[skip..])
					{
						//log_trace!("ent={:?}", ent);
						// TODO: Need to handle this _FAR_ better
						if let SuspItem::AlternateName(0, new_name) = ent {
							name = new_name;
						}
					}
				}

				Ok(Some(DirEnt {
					this_ofs: cur_ofs,
					next_ofs: self.ofs,
					flags: ent[25],
					start: LittleEndian::read_u32(&ent[2..]),
					size: LittleEndian::read_u32(&ent[10..]),
					name: name,
					sys_use: su,
					}))
			}
		}
	}
}

struct SuspIterator<'a>(&'a [u8]);

#[derive(Debug)]
#[allow(dead_code)]
enum SuspItem<'a>
{
	// SUSP Base
	ContinuationEntry(u32, u32, u32),
	Pad(&'a [u8]),
	Identifer,
	//End,
	
	// RockRidge
	RockRidge(u8),
	PosixMode {
		mode: u32,
		n_links: u32,
		uid: u32,
		gid: u32,
		serial_number: u32,
		},
	AlternateName(u8, &'a [u8]),
	Timestamps {
		flags: u8,
		data: &'a [u8],
		},

	Unknown([u8; 2], u8, &'a[u8]),
}

impl<'a> Iterator for SuspIterator<'a>
{
	type Item = SuspItem<'a>;
	fn next(&mut self) -> Option<SuspItem<'a>>
	{
		if self.0.len() == 0 {
			None
		}
		else if self.0.len() < 4 {
			None
		}
		else {
			let tag = [self.0[0], self.0[1]];
			let len = self.0[2] as usize;
			let ver = self.0[3];
			if len < 4 {
				return None;
			}
			if self.0.len() < len {
				return None;
			}
			let data = &self.0[4..len];

			self.0 = &self.0[len..];
			
			log_debug!("tag = {}{} - data={} [{:?}]", tag[0] as char, tag[1] as char, len-4, data);
			Some(match &tag[..]
				{
				b"ST" => return None,	// Terminated
				b"SP" => SuspItem::Identifer,
				b"PD" => SuspItem::Pad(data),
				b"CE" => {
					if data.len() < 3*8 { return None; }
					SuspItem::ContinuationEntry(
						LittleEndian::read_u32(&data[0..]),
						LittleEndian::read_u32(&data[8..]),
						LittleEndian::read_u32(&data[16..])
						)
					},
				b"RR" => {
					if data.len() < 1 { return None; }
					SuspItem::RockRidge(data[0])
					},
				b"PX" => {
					if data.len() < 4*8 { return None; }
					SuspItem::PosixMode {
						mode:    LittleEndian::read_u32(&data[0..]),
						n_links: LittleEndian::read_u32(&data[8..]),
						uid:     LittleEndian::read_u32(&data[16..]),
						gid:     LittleEndian::read_u32(&data[24..]),
						serial_number: if data.len() >= 32+8 { LittleEndian::read_u32(&data[32..]) } else { 0 },
						}
					},
				b"TF" => {
					if data.len() < 1 { return None; }
					SuspItem::Timestamps {
						flags: data[0],
						data: &data[1..],
						}
					},
				//b"SL" => SuspItem::Symlink(),
				b"NM" => {
					if data.len() < 1 { return None; }
					SuspItem::AlternateName(data[0], &data[1..])
					},
				_ => SuspItem::Unknown(tag, ver, data),
				})
		}
	}
}
