// "Tifflin" Kernel - ISO9660 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_iso9660/lib.rs
#![feature(linkage)]
#![feature(clone_from_slice)]
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
			try!(vol.read_blocks(32*1024 / bs, &mut block));
			block
			};
		if &blk[1..6] == b"CD001" {
			Ok(1)
		}
		else {
			Ok(0)
		}
	}
	fn mount(&self, vol: VolumeHandle) -> vfs::Result<Box<mount::Filesystem>> {
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
			try!(vol.read_blocks((sector*scale) as u64, &mut block));
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
		
		Ok( Box::new( Instance(
			// SAFE: Stored in a box, and not moved out.
			unsafe { ArefInner::new( InstanceInner {
				vh: ::block_cache::CacheHandle::new(vol),
				lb_size: lb_size as usize,
				root_lba: root_lba,
				root_size: root_size,
				} ) }
			) ) )
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
			let blk = match self.get_sector(sector as u32)
				{
				Ok(v) => v,
				Err(_) => return None,
				};
			let mut it = DirSector::new(blk, ofs as usize);
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
struct Sector(::block_cache::CachedBlockHandle,u16,u16);
impl ::core::ops::Deref for Sector {
	type Target = [u8];
	fn deref(&self) -> &[u8] {
		&self.0.data()[self.1 as usize ..][.. self.2 as usize]
	}
}
impl InstanceInner
{
	/// Read a sector from the disk into the provided buffer
	fn read_sector(&self, sector: u32, buf: &mut [u8]) -> Result<(), storage::IoError> {
		assert_eq!(buf.len(), self.lb_size);
		// - Will be round, Driver::mount() ensures this
		let hwsects_per_lb = self.lb_size / self.vh.block_size();
		let hwsector = sector as usize * hwsects_per_lb;
		try!( self.vh.read_blocks( hwsector as u64, buf) );
		Ok( () )
		
	}
	/// Read a metadata sector via a cache
	fn get_sector(&self, sector: u32) -> Result<Sector, storage::IoError> {
		assert!(sector > 0);
		
		let blk = try!(self.vh.get_block(sector as u64));
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
}
impl node::File for File
{
	fn size(&self) -> u64 {
		self.size as u64
	}
	fn truncate(&self, _newsize: u64) -> node::Result<u64> {
		Err(node::IoError::ReadOnly)
	}
	fn clear(&self, _ofs: u64, _size: u64) -> node::Result<()> {
		Err(node::IoError::ReadOnly)
	}
	fn read(&self, ofs: u64, buf: &mut [u8]) -> node::Result<usize> {
		if ofs > self.size as u64 {
			Err(node::IoError::OutOfRange)
		}
		else {
			let len = ::core::cmp::min( buf.len(),  (self.size as u64 - ofs) as usize );
			
			let (sector, ofs) = ::kernel::lib::num::div_rem(ofs, self.fs.lb_size as u64);
			let mut sector = sector as u32;
			let ofs = ofs as usize;

			let mut read = 0;
			// 1. Leading
			if ofs > 0 {
				let mut tmp = vec![0u8; self.fs.lb_size];
				try!(self.fs.read_sector(self.first_lba + sector, &mut tmp));

				sector += 1;
				read += buf[read..].clone_from_slice(&tmp[ofs..]);
			}

			// 2. Inner
			if len - read >= self.fs.lb_size {
				let sectors = (len - read) / self.fs.lb_size;
				let bytes = sectors * self.fs.lb_size;
				try!(self.fs.read_sector(self.first_lba + sector, &mut buf[read..][..bytes]));
				sector += sectors as u32;
				read += bytes;
			}

			// 3. Trailing
			if read < len {
				let mut tmp = vec![0; self.fs.lb_size];
				try!(self.fs.read_sector(self.first_lba + sector, &mut tmp));

				buf[read..].clone_from_slice(&tmp);
			}

			Ok( len )
		}
	}
	fn write(&self, _ofs: u64, _buf: &[u8]) -> node::Result<usize> {
		Err(node::IoError::ReadOnly)
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
}
impl node::Dir for Dir
{
	fn lookup(&self, name: &ByteStr) -> node::Result<node::InodeId>
	{
		for sector in 0 .. ::kernel::lib::num::div_up(self.size, self.fs.lb_size as u32)
		{
			let mut it = DirSector::new(try!(self.fs.get_sector(self.first_lba + sector)), 0); 

			while let Some(ent) = try!(it.next())
			{
				if ent.name == name.as_bytes()
				{
					let inode = (self.first_lba + sector) as u64 * self.fs.lb_size as u64 + ent.this_ofs as u64;
					return Ok( inode );
				}
			}
		}

		Err(node::IoError::NotFound)
	}
	fn read(&self, ofs: usize, callback: &mut node::ReadDirCallback) -> node::Result<usize>
	{
		let max_sectors = ::kernel::lib::num::div_up(self.size, self.fs.lb_size as u32);
		let end_ofs = self.size as usize % self.fs.lb_size;

		let (sector, mut ofs) = (ofs / self.fs.lb_size, ofs % self.fs.lb_size);
		
		for sector in sector as u32 .. max_sectors
		{
			let mut it = DirSector::new( try!(self.fs.get_sector(self.first_lba + sector)),  ofs );
			ofs = 0;

			while let Some(ent) = try!(it.next())
			{
				if ent.name.len() > 0
				{
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
		Err( node::IoError::ReadOnly )
	}
	fn link(&self, _name: &ByteStr, _inode: node::InodeId) -> node::Result<()> {
		// ISO9660 is readonly
		Err( node::IoError::ReadOnly )
	}
	fn unlink(&self, _name: &ByteStr) -> node::Result<()> {
		// ISO9660 is readonly
		Err( node::IoError::ReadOnly )
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
}

impl<'a> DirEnt<'a>
{
}

struct DirSector {
	data: Sector,
	ofs: usize
}

impl DirSector
{
	pub fn new(data: Sector, start_ofs: usize) -> DirSector
	{
		DirSector {
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
				return Err(node::IoError::Corruption);
			}
			else if self.ofs + len > self.data.len() {
				log_warning!("Consistency error in filesystem (dir entry spans sectors)");
				return Err(node::IoError::Corruption);
			}
			else {
				let ent = &self.data[self.ofs ..][.. len];
				self.ofs += len;
				
				let namelen = ent[32] as usize;
				if 33 + namelen > len {
					log_warning!("Name overruns end of entry");
					return Err(node::IoError::Corruption);
				}

				Ok(Some(DirEnt {
					this_ofs: cur_ofs,
					next_ofs: self.ofs,
					flags: ent[25],
					start: LittleEndian::read_u32(&ent[2..]),
					size: LittleEndian::read_u32(&ent[10..]),
					name: &ent[33 ..][.. namelen],
					}))
			}
		}
	}
}

