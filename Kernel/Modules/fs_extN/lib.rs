// "Tifflin" Kernel - ext2/3/4 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_extN/lib.rs
//! Ext2/3/4 filesystem driver
#![feature(linkage)]
#![feature(clone_from_slice)]
#![no_std]

use kernel::prelude::*;
use kernel::vfs;
use kernel::metadevs::storage::VolumeHandle;

#[macro_use]
extern crate kernel;

extern crate buffered_volume;

module_define!{FS_EXTN, [VFS], init}

mod ondisk;
mod inodes;

mod dir;
mod file;
mod instance;

fn init()
{
	::core::mem::forget( vfs::mount::DriverRegistration::new("extN", &S_DRIVER) )
}

static S_DRIVER: Driver = Driver;
struct Driver;

impl vfs::mount::Driver for Driver
{
	fn detect(&self, vol: &VolumeHandle) -> vfs::Result<usize> {
		let bs = vol.block_size() as u64;

		// The superblock exists at offset 1024 in the volume, no matter the on-disk block size
		let superblock_idx = 1024 / bs;
		let superblock_ofs = (1024 % bs) as usize;

		let blk = {
			let mut block: Vec<u8> = vec![0; ::core::cmp::max(1024, bs) as usize];
			try!(vol.read_blocks(superblock_idx, &mut block));
			block
			};

		// Superblock magic is 0xEF53 in little-endian
		if &blk[superblock_ofs + ondisk::S_MAGIC_OFS..][..2] == b"\x53\xEF" {
			Ok(2)
		}
		else {
			Ok(0)
		}
	}
	fn mount(&self, vol: VolumeHandle) -> vfs::Result<Box<vfs::mount::Filesystem>> {
		Ok( try!(instance::Instance::new_boxed(vol)) )
	}
}

