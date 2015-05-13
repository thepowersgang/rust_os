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

use kernel::vfs::{mount, node};
use kernel::metadevs::storage::VolumeHandle;

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
	fn detect(&self, _vol: &VolumeHandle) -> usize {
		todo!("detect()")
	}
	fn mount(&self, vol: VolumeHandle) -> Result<Box<mount::Filesystem>, ()> {
		todo!("mount()")
	}
}

