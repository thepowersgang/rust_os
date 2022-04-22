// "Tifflin" Kernel - ext3/3/4 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_extN/lib.rs
//! Ext2/3/4 filesystem driver
#![feature(linkage)]
#![feature(stmt_expr_attributes)]	// For local "constant"s
#![no_std]

use kernel::prelude::*;
use kernel::vfs;
use kernel::metadevs::storage::VolumeHandle;

#[macro_use]
extern crate kernel;

extern crate block_cache;

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
	| ::ondisk::FEAT_COMPAT_EXT_ATTR	// Extended attributes
	| ::ondisk::FEAT_COMPAT_RESIZE_INODE	// Extra space was allocated for resizing the filesystem
	;
/// Read-only features: Missing features stop write support
const SUPPORTED_RDO_FEATURES: u32 = 0
	| ::ondisk::FEAT_RO_COMPAT_SPARSE_SUPER	// Enables storing SB backups at group 0, 3^n, 5^n, and 7^n
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
			::kernel::futures::block_on(vol.read_blocks(superblock_idx, ::kernel::lib::as_byte_slice_mut(&mut block[..])))?;
			block
			};
		let sb = &::ondisk::Superblock::from_slice(&blk[superblock_ofs / 4 ..][..1024/4]);

		if sb.data.s_magic == 0xEF53 {
			use instance::FeatureState;
			match ::instance::Instance::check_features(vol.name(), &sb)
			{
			FeatureState::AllOk => Ok(3),
			// Lower binding strength if slightly incompatible
			FeatureState::Reduced(_) => Ok(2),
			FeatureState::ReadOnly(_) => Ok(2),
			// Can't bind
			FeatureState::Incompatible(_) => Ok(0),
			}
		}
		else {
			Ok(0)
		}
	}
	fn mount(&self, vol: VolumeHandle, mounthandle: vfs::mount::SelfHandle) -> vfs::Result<Box<dyn vfs::mount::Filesystem>> {
		Ok( try!(instance::Instance::new_boxed(vol, mounthandle)) )
	}
}

