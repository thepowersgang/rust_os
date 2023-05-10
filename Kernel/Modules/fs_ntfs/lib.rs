// "Tifflin" Kernel - NTFS Driver
// - By John Hodge (Mutabah/thePowersGang)
//
// Modules/fs_ntfs/lib.rs
//! NTFS filesystem driver
#![feature(linkage)]
#![no_std]

use kernel::metadevs::storage::VolumeHandle;
use kernel::prelude::*;

#[macro_use]
extern crate kernel;

extern crate block_cache;

module_define! {FS_NTFS, [VFS], init}

mod instance;
mod helpers;
mod ondisk;
use helpers::MftEntryIdx;

fn init() {
	::core::mem::forget( vfs::mount::DriverRegistration::new("ntfs", &Driver) )
}

struct Driver;
impl vfs::mount::Driver for Driver
{
	fn detect(&self, vol: &VolumeHandle) -> vfs::Result<usize> {
		// Read the first block, check if it passes various checks on the NTFS boot sector
		let bs = {
			let mut block = vec![0; ::core::cmp::max(512, vol.block_size() as usize)];
			::kernel::futures::block_on(vol.read_blocks(0, &mut block[..]))?;
			ondisk::Bootsector::from_slice(&block)
			};

		if is_valid_bootsector(&bs) {
			Ok(1)
		}
		else {
			Ok(0)
		}
	}
	fn mount(&self, vol: VolumeHandle, mount_handle: vfs::mount::SelfHandle) -> vfs::Result<Box<dyn vfs::mount::Filesystem>> {
		let bs = {
			let mut block = vec![0; ::core::cmp::max(512, vol.block_size() as usize)];
			::kernel::futures::block_on(vol.read_blocks(0, &mut block[..]))?;
			ondisk::Bootsector::from_slice(&block)
			};
		if !is_valid_bootsector(&bs) {
			return Err(vfs::Error::TypeMismatch);
		}

		Ok(instance::Instance::new(vol, bs, mount_handle)?)
	}
}
fn is_valid_bootsector(bs: &ondisk::Bootsector) -> bool {
	if bs.system_id != *b"NTFS    " { return false; }

	// Snity-check various parameters
	// - BPS field must be 512 or more, and be a power of two
	if bs.bytes_per_sector < 512 {
		log_error!("is_valid_bootsector: bs.bytes_per_sector({}) < 512", {bs.bytes_per_sector});
		return false;
	}
	if (bs.bytes_per_sector - 1) & bs.bytes_per_sector != 0 {
		log_error!("is_valid_bootsector: bs.bytes_per_sector({:#x}) not a power of two", {bs.bytes_per_sector});
		return false;
	}
	// - Count fields should be non-zero
	if bs.sectors_per_cluster == 0 {
		log_error!("is_valid_bootsector: bs.sectors_per_cluster == 0");
		return false;
	}
	if bs.mft_record_size.raw() == 0 {
		log_error!("is_valid_bootsector: bs.mft_record_size == 0");
		return false;
	}
	if bs.index_record_size.raw() == 0 {
		log_error!("is_valid_bootsector: bs.index_record_size == 0");
		return false;
	}

	// - MFTs must be within bounds
	let nclusters = bs.total_sector_count / bs.sectors_per_cluster as u64;
	if bs.mft_start >= nclusters {
		log_error!("is_valid_bootsector: bs.mft_start({:#x}) >= nclusters({:#x})", {bs.mft_start}, nclusters);
		return false;
	}
	if bs.mft_mirror_start >= nclusters {
		log_error!("is_valid_bootsector: bs.mft_mirror_start({:#x}) >= nclusters({:#x})", {bs.mft_mirror_start}, nclusters);
		return false;
	}

	true
}

