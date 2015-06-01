// "Tifflin" Kernel - ISO9660 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_iso9660/lib.rs
#![feature(no_std,core,linkage)]
#![no_std]
#[macro_use] extern crate core;
#[macro_use] extern crate kernel;
use kernel::prelude::*;

use kernel::vfs::{self, mount, node};
use kernel::metadevs::storage::{self,VolumeHandle,SizePrinter};
use kernel::lib::mem::aref::{ArefInner,ArefBorrow};
use kernel::lib::byteorder::{ByteOrder,LittleEndian};

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
	pv: VolumeHandle,
	lb_size: usize,
	root_lba: u32,
	root_size: u32,
}

fn init()
{
	let h = mount::DriverRegistration::new("iso9660", &S_DRIVER);
	// TODO: Remember the registration for unloading
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
		if 2048 % vol.block_size() != 0 {
			return Err( vfs::Error::Unknown("Can't mount ISO9660 with sector size not a factor of 2048"/*, vol.block_size()*/) );
		}
		let scale = 2048 / vol.block_size();
		
		let mut block: Vec<u8> = (0 .. 2048).map(|_|0).collect();
		for sector in (16 .. )
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
		::kernel::logging::hex_dump("ISO966 PVD", &block);
		
		// Obtain the logical block size (different from medium sector size)
		let lb_size = LittleEndian::read_u16(&block[128..]);
		// Extract the root directory entry
		// - We want the LBA and byte length
		let root_lba  = LittleEndian::read_u32(&block[156+ 2..]);
		let root_size = LittleEndian::read_u32(&block[156+10..]);
		
		Ok( Box::new( Instance(
			unsafe { ArefInner::new( InstanceInner {
				pv: vol,
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
		todo!("Instance::root_inode");
	}
	fn get_node_by_inode(&self, id: node::InodeId) -> Option<node::Node> {
		todo!("Instance::get_node_by_inode");
	}

}


