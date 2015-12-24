// "Tifflin" Kernel - ext2/3/4 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_extN/ondisk.rs
//! On-disk structures

pub const S_MAGIC_OFS: usize = (3*4*4 + 2*4);

macro_rules! pod_impls {
	($t:ty) => {
		impl Copy for $t {
		}
		impl Clone for $t {
			fn clone(&self) -> Self { *self }
		}
		impl Default for $t {
			fn default() -> Self {
				// SAFE: Copy types are safe to zero... well, except &, but meh
				unsafe { ::core::mem::zeroed() }
			}
		}
		impl $t {
			#[allow(dead_code)]
			pub fn from_slice(r: &mut [u32]) -> &mut Self {
				assert_eq!(r.len() * 4, ::core::mem::size_of::<Self>() );
				// SAFE: Alignment is correct, (max is u32), size checked
				unsafe {
					let p = r.as_ptr() as *mut Self;
					&mut *p
				}
			}
		}
	};
}

#[repr(C)]
pub struct Superblock
{
	pub data: SuperblockData,
	pub _s_reserved: [u32; 235],	// Padding to the end of the block
}
#[repr(C)]
pub struct SuperblockData
{
	pub s_inodes_count: u32,		// Inodes count
	pub s_blocks_count: u32,		// Blocks count
	pub s_r_blocks_count: u32,	// Reserved blocks count
	pub s_free_blocks_count: u32,	// Free blocks count

	pub s_free_inodes_count: u32,	// Free inodes count
	pub s_first_data_block: u32,	// First Data Block
	pub s_log_block_size: u32,	// Block size
	pub s_log_frag_size: i32,	// Fragment size

	pub s_blocks_per_group: u32,	// Number Blocks per group
	pub s_frags_per_group: u32,	// Number Fragments per group
	pub s_inodes_per_group: u32,	// Number Inodes per group
	pub s_mtime: u32,			// Mount time

	pub s_wtime: u32,			// Write time
	pub s_mnt_count: u16,		// Mount count
	pub s_max_mnt_count: i16,	// Maximal mount count
	pub s_magic: u16,			// Magic signature
	pub s_state: u16,			// File system state
	pub s_errors: u16,			// Behaviour when detecting errors
	pub s_pad: u16,				// Padding

	pub s_lastcheck: u32,		// time of last check
	pub s_checkinterval: u32,	// max. time between checks
	pub s_creator_os: u32,		// Formatting OS
	pub s_rev_level: u32,		// Revision level

	pub s_def_resuid: u16,		// Default uid for reserved blocks
	pub s_def_resgid: u16,		// Default gid for reserved blocks
}

impl Copy for SuperblockData {}
impl Clone for SuperblockData {
	fn clone(&self) -> Self { *self }
}
pod_impls!{ Superblock }

#[repr(C)]
pub struct Inode
{
	pub i_mode: u16,	// File mode
	pub i_uid: u16, 	// Owner Uid
	pub i_size: u32,	// Size in bytes
	pub i_atime: u32,	// Access time
	pub i_ctime: u32,	// Creation time
	pub i_mtime: u32,	// Modification time
	pub i_dtime: u32,	// Deletion Time
	pub i_gid: u16, 	// Group Id
	pub i_links_count: u16,	// Links count
	pub i_blocks: u32,	// Number of blocks allocated for the file
	pub i_flags: u32,	// File flags
	pub _osd1: u32, 	// OS Dependent #1
	pub i_block: [u32; 15],	// Pointers to blocks
	pub i_version: u32,	// File version (for NFS)
	pub i_file_acl: u32,	// File ACL
	pub i_dir_acl: u32,	// Directory ACL / Extended File Size
	pub i_faddr: u32,	// Fragment address
	pub _osd2: [u32; 3],	// OS Dependent #2 (Typically fragment info)
}
pod_impls!{ Inode }

pub const S_IFMT: u16 = 0xF000;	// Format Mask
pub const S_IFSOCK: u16 = 0xC000;	// Socket
pub const S_IFLNK: u16 = 0xA000;	// Symbolic Link
pub const S_IFREG: u16 = 0x8000;	// Regular File
pub const S_IFBLK: u16 = 0x6000;	// Block Device
pub const S_IFDIR: u16 = 0x4000;	// Directory
pub const S_IFCHR: u16 = 0x2000;	// Character Device
pub const S_IFIFO: u16 = 0x1000;	// FIFO

pub const S_ISUID: u16 = 0x0800;	// SUID
pub const S_ISGID: u16 = 0x0400;	// SGID
pub const S_ISVTX: u16 = 0x0200;	// sticky bit
pub const S_IRWXU: u16 = 0700;	// user access rights mask
pub const S_IRUSR: u16 = 0400;	// Owner Read
pub const S_IWUSR: u16 = 0200;	// Owner Write
pub const S_IXUSR: u16 = 0100;	// Owner Execute
pub const S_IRWXG: u16 = 0070;	// Group Access rights mask
pub const S_IRGRP: u16 = 0040;	// Group Read
pub const S_IWGRP: u16 = 0020;	// Group Write
pub const S_IXGRP: u16 = 0010;	// Group Execute
pub const S_IRWXO: u16 = 0007;	// Global Access rights mask
pub const S_IROTH: u16 = 0004;	// Global Read
pub const S_IWOTH: u16 = 0002;	// Global Write
pub const S_IXOTH: u16 = 0001;	// Global Execute

#[repr(C)]
pub struct GroupDesc
{
	pub bg_block_bitmap: u32,	// Blocks bitmap block
	pub bg_inode_bitmap: u32,	// Inodes bitmap block
	pub bg_inode_table: u32,	// Inodes table block
	pub bg_free_blocks_count: u16,	// Free blocks count
	pub bg_free_inodes_count: u16,	// Free inodes count
	pub bg_used_dirs_count: u16,	// Directories count
	pub bg_pad: u16,	// Padding
	pub bg_reserved: [u32; 3],	// Reserved
}
pod_impls!{ GroupDesc }

