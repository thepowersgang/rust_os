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

/// Optional Features: Doesn't stop Read+Write, but might confuse other systems or be inefficient
const SUPPORTED_OPT_FEATURES: u32 = 0
	;
/// Read-only features: Missing features stop write support
const SUPPORTED_RDO_FEATURES: u32 = 0
	;
/// Required Features: Missing features prevent mounting
const SUPPORTED_REQ_FEATURES: u32 = 0
	| ::ondisk::FEAT_INCOMPAT_FILETYPE	// DirEnt.d_name_len restricted to 1 byte and extra byte used for file type
	;

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
			let mut block: Vec<u32> = vec![0; (::core::cmp::max(1024, bs) / 4) as usize];
			try!(vol.read_blocks(superblock_idx, ::kernel::lib::as_byte_slice_mut(&mut block[..])));
			block
			};
		let sb = &::ondisk::Superblock::from_slice(&blk[superblock_ofs / 4 ..][..1024/4]);

		if sb.data.s_magic == 0xEF53 {
			// Legacy (no feature flags)
			if sb.data.s_rev_level == 0 {
				Ok(3)
			}
			else {
				let unsupported_req = sb.ext.s_feature_incompat & !SUPPORTED_REQ_FEATURES;
				let unsupported_rdo = sb.ext.s_feature_ro_compat & !SUPPORTED_RDO_FEATURES;
				let unsupported_opt = sb.ext.s_feature_compat & !SUPPORTED_OPT_FEATURES;
				if unsupported_req != 0 {
					log_warning!("Volume `{}` uses incompatible required features (unsupported bits {:#x})", vol.name(), unsupported_req);
					Ok(0)
				}
				else if unsupported_rdo != 0 {
					// Read-only
					log_warning!("Volume `{}` uses incompatible read-write features (unsupported bits {:#x})", vol.name(), unsupported_rdo);
					Ok(2)
				}
				else if unsupported_opt != 0 {
					// Can read and write, but may confuse other systems
					log_warning!("Volume `{}` uses incompatible optional features (unsupported bits {:#x})", vol.name(), unsupported_rdo);
					Ok(2)
				}
				else {
					// Fully supported
					Ok(3)
				}
			}
		}
		else {
			Ok(0)
		}
	}
	fn mount(&self, vol: VolumeHandle) -> vfs::Result<Box<vfs::mount::Filesystem>> {
		Ok( try!(instance::Instance::new_boxed(vol)) )
	}
}

