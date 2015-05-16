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
use kernel::metadevs::storage::{self,VolumeHandle};

module_define!{FS_FAT, [VFS], init}

struct Driver;

static S_DRIVER: Driver = Driver;

fn init()
{
	let h = mount::DriverRegistration::new("fat", &S_DRIVER);
	unsafe { ::core::mem::forget(h); }
}

impl mount::Driver for Driver
{
	fn detect(&self, vol: &VolumeHandle) -> vfs::Result<usize> {
		use kernel::lib::byteorder::{ReadBytesExt,LittleEndian};
		
		let mut bs = [0u8; 512];
		try!( vol.read_blocks(0, &mut bs) );
		
		let bps = (&bs[0x0b..]).read_u16::<LittleEndian>().unwrap();
		let spc = (&bs[0x0d..]).read_u8().unwrap();
		let media_desc = (&bs[0x15..]).read_u8().unwrap();
		
		
		if bps == 0 || spc == 0 || media_desc < 0xf0 {
			Ok(0)
		}
		else {
			Ok(1)
		}
	}
	fn mount(&self, vol: VolumeHandle) -> vfs::Result<Box<mount::Filesystem>> {
		todo!("mount()")
	}
}

