// "Tifflin" Kernel - ext2/3/4 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_extN/ondisk.rs
//! On-disk structures
#![allow(dead_code)]

pub const S_MAGIC_OFS: usize = 3*4*4 + 2*4;

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
	};
}

macro_rules! def_from_slice {
	($t:ty) => {
		impl $t {
			#[allow(dead_code)]
			pub fn from_slice(r: &[u32]) -> &Self {
				assert_eq!(r.len() * 4, ::core::mem::size_of::<Self>() );
				// SAFE: Alignment is correct, (max is u32), size checked
				unsafe {
					let p = r.as_ptr() as *const Self;
					&*p
				}
			}
		}
	};
}

// Packed is required because the base data is an odd number of u32s long, and extension has u64s
#[repr(packed,C)]
pub struct Superblock
{
	pub data: SuperblockData,
	pub ext: SuperblockDataExt,
	pub _s_reserved: [u32; 98],
	pub s_checksum: u32,
}
#[allow(dead_code)]
// SAFE: Never called, and does POD transmutes
fn _sb_size() { unsafe {
	use core::mem::transmute;
	let _: [u32; 0x54/4] = transmute(SuperblockData::default());
	let _: [u32; (0x274-0x54)/4] = transmute(SuperblockDataExt::default());
	let _: [u32; 1024/4] = transmute(Superblock::default());
} }
pod_impls!{ Superblock }
def_from_slice!{ Superblock }

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
	pub s_log_cluster_size: i32,	// Cluster size [FEAT_RO_COMPAT_BIGALLOC]

	pub s_blocks_per_group: u32,	// Number Blocks per group
	pub s_clusters_per_group: u32,	// Number of clusters per group
	pub s_inodes_per_group: u32,	// Number Inodes per group

	pub s_mtime: u32,			// Mount time
	pub s_wtime: u32,			// Write time

	pub s_mnt_count: u16,		// Mount count
	pub s_max_mnt_count: i16,	// Maximal mount count

	pub s_magic: u16,			// Magic signature
	pub s_state: u16,			// File system state
	pub s_errors: u16,			// Behaviour when detecting errors
	pub s_minor_rev_level: u16,				// Padding

	pub s_lastcheck: u32,		// time of last check
	pub s_checkinterval: u32,	// max. time between checks

	pub s_creator_os: u32,		// Formatting OS
	pub s_rev_level: u32,		// Revision level

	pub s_def_resuid: u16,		// Default uid for reserved blocks
	pub s_def_resgid: u16,		// Default gid for reserved blocks
}
pod_impls!{ SuperblockData }

#[repr(C)]
pub struct SuperblockDataExt
{
	// The following fields are only valid if s_rev_level > 0
	pub s_first_ino: u32,	// First valid inode
	pub s_inode_size: u16,	// Size of inode structure in bytes
	pub s_block_group_nr: u16,	// Block group number of this superblock (for backups)

	/// Compatible feature set flags (see FEAT_COMPAT_*). Can mount full RW if unknown
	pub s_feature_compat: u32,
	/// Incompatible feature set flags (see FEAT_INCOMPAT_*). Can't mount if unknown
	pub s_feature_incompat: u32,
	/// Read-only compatible feature set flags (see FEAT_RO_COMPAT_*). Can read but can't write if unknown
	pub s_feature_ro_compat: u32,

	/// 128-bit volume UUID
	pub s_uuid: [u8; 16],
	/// Volume label
	pub s_volume_name: [u8; 16],
	/// Last mounted directory
	pub s_last_mounted: [u8; 64],
	
	// FEAT_COMPAT_DIR_PREALLOC
	pub s_prealloc_blocks: u8,
	pub s_prealloc_dir_blocks: u8,
	pub s_reserved_gdt_blocks: u16,

	// FEAT_COMPAT_HAS_JOURNAL
	pub s_journal_uuid: [u8; 16],
	/// Inode number of the journal
	pub s_journal_inum: u32,
	/// Journal device number if an external journal is in use (FEAT_INCOMPAT_JOURNAL_DEV)
	pub s_journal_dev: u32,


	pub s_last_orphan: u32,
	pub s_hash_seed: [u32; 4],
	pub s_def_hash_version: u8,
	pub s_jnl_backup_type: u8,

	/// [FEAT_INCOMPAT_64BIT] Group descriptor size
	pub s_desc_size: u16,
	pub s_default_mount_opts: u32,
	/// [FEAT_INCOMPAT_META_BG] First metadata block group
	pub s_first_meta_bg: u32,
	pub s_mkfs_time: u32,
	pub s_jnl_blocks: [u32; 15+2],
	// FEAT_INCOMPAT_64BIT
	pub s_blocks_count_hi: u32,
	pub s_r_blocks_count_hi: u32,
	pub s_free_blocks_count_hi: u32,
	pub s_min_extra_isize: u16,
	pub s_want_extra_isize: u16,

	pub s_flags: u32,
	pub s_raid_stride: u16,
	pub s_mmp_interval: u16,
	pub s_mmp_block: u64,
	pub s_raid_stripe_width: u32,
	pub s_log_groups_per_flex: u8,
	pub s_checksum_type: u8,
	pub _s_reserved_pad: u16,

	pub s_kbytes_written: u64,
	// Snapshots
	pub s_snapshot_inum: u32,
	pub s_snapshot_id: u32,
	pub s_snapshot_r_blocks_count: u64,
	pub s_snapshot_list: u32,
	// Error tracking
	pub s_error_count: u32,
	pub s_first_error_time: u32,
	pub s_first_error_ino: u32,
	pub s_first_error_block: u64,
	pub s_first_error_func: [u8; 32],
	pub s_first_error_line: u32,
	// - Most recent error
	pub s_last_error_time: u32,
	pub s_last_error_ino: u32,
	pub s_last_error_line: u32,
	pub s_last_error_block: u64,
	pub s_last_error_func: [u8; 32],

	pub s_mount_opts: [u8; 64],
	pub s_usr_quota_inum: u32,
	pub s_grp_quota_inum: u32,
	pub s_overhead_blocks: u32,
	/// [FEAT_COMPAT_SPARSE_SUPER2]
	pub s_backup_bgs: [u32; 2],
	pub s_encrypt_algos: [u8; 4],
	pub s_encrypt_pw_salt: [u8; 16],
	/// Inode number of `lost+found` folder
	pub s_lpf_ino: u32,
	/// [FEAT_RO_COMPAT_PROJECT] Project quota inode
	pub s_prj_quota_inum: u32,
	pub s_checksum_seed: u32,
}
pod_impls!{ SuperblockDataExt }

macro_rules! def_bitset {
	( $($v:expr => $name:ident,)* ) => {
		$( pub const $name: u32 = 1 << $v; )*
		};
}

def_bitset! {
	 0 => FEAT_INCOMPAT_COMPRESSION,
	 1 => FEAT_INCOMPAT_FILETYPE, 
	 2 => FEAT_INCOMPAT_RECOVER, 
	 3 => FEAT_INCOMPAT_JOURNAL_DEV,
	 4 => FEAT_INCOMPAT_META_BG,
	 5 => _FEAT_INCOMPAT_UNUSED,
	 6 => FEAT_INCOMPAT_EXTENTS,	// Some files use extents
	 7 => FEAT_INCOMPAT_64BIT,	// 64-bit block count
	 8 => FEAT_INCOMPAT_MMP,	// Multiple mount protection
	 9 => FEAT_INCOMPAT_FLEX_BG,	// Flexible block groups
	10 => FEAT_INCOMPAT_EA_MODE,	// Inodes used for large extneded attributes
	12 => FEAT_INCOMPAT_DIRDATA,	// Directory entry can contain data
	13 => FEAT_INCOMPAT_CSUM_SEED,
	14 => FEAT_INCOMPAT_LARGEDIR,	// Large Directories (>2GB) or 3-level htree
	15 => FEAT_INCOMPAT_INLINE_DATA,	// Data stored in inode
	16 => FEAT_INCOMPAT_ENCRYPT,	// Encrypted inodes present
}

def_bitset! {
	 0 => FEAT_RO_COMPAT_SPARSE_SUPER,
	 1 => FEAT_RO_COMPAT_LARGE_FILE,
	 2 => FEAT_RO_COMPAT_BTREE_DIR,
	 3 => FEAT_RO_COMPAT_HUGE_FILE,
	 4 => FEAT_RO_COMPAT_GDT_CSUM,
	 5 => FEAT_RO_COMPAT_DIR_NLINK,
	 6 => FEAT_RO_COMPAT_EXTRA_ISIZE,
	 7 => FEAT_RO_COMPAT_HAS_SNAPSHOT,
	 8 => FEAT_RO_COMPAT_QUOTA,
	 9 => FEAT_RO_COMPAT_BIGALLOC,
	10 => FEAT_RO_COMPAT_METADATA_CSUM,
	11 => FEAT_RO_COMPAT_REPLICA,
	12 => FEAT_RO_COMPAT_READONLY,
	13 => FEAT_RO_COMPAT_PROJECT,
}

pub const FEAT_COMPAT_DIR_PREALLOCT: u32 = 1 << 0;	// Directory Preallocation
pub const FEAT_COMPAT_IMAGIC_INODES: u32 = 1 << 1;	// ?
pub const FEAT_COMPAT_HAS_JOURNAL  : u32 = 1 << 2;
pub const FEAT_COMPAT_EXT_ATTR     : u32 = 1 << 3;	// Extended attributes
pub const FEAT_COMPAT_RESIZE_INODE : u32 = 1 << 4;	// Reserved GDT blocks for expansion
pub const FEAT_COMPAT_DIR_INDEX    : u32 = 1 << 5;	// Directory indicies [?]
pub const FEAT_COMPAT_LAZY_BG      : u32 = 1 << 6;
pub const FEAT_COMPAT_EXCLUDE_INODE: u32 = 1 << 7;
pub const FEAT_COMPAT_EXCLUDE_BITMAP:u32 = 1 << 8;
pub const FEAT_COMPAT_SPARSE_SUPER2: u32 = 1 << 9;

#[repr(C)]
#[derive(Debug)]
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
#[repr(C)]
#[derive(Debug)]
pub struct InodeExtra
{
	// FEAT_COMPAT_EXT_ATTR
	pub i_extra_size: u16,	// Size of extra fields
	pub i_checksum_hi: u16,
	pub i_ctime_extra: u32,	// Sub-section precision on CTime
	pub i_mtime_extra: u32,
	pub i_atime_extra: u32,
	pub i_crtime: u32,	// File creation time
	pub i_crtime_extra: u32,
	pub i_version_hi: u32,
	pub i_projid: u32,
}
pod_impls!{ Inode }
//def_from_slice!{ Inode }

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
pub const S_IRWXU: u16 =  0o700;	// user access rights mask
pub const S_IRUSR: u16 =  0o400;	// Owner Read
pub const S_IWUSR: u16 =  0o200;	// Owner Write
pub const S_IXUSR: u16 =  0o100;	// Owner Execute
pub const S_IRWXG: u16 =  0o070;	// Group Access rights mask
pub const S_IRGRP: u16 =  0o040;	// Group Read
pub const S_IWGRP: u16 =  0o020;	// Group Write
pub const S_IXGRP: u16 =  0o010;	// Group Execute
pub const S_IRWXO: u16 =  0o007;	// Global Access rights mask
pub const S_IROTH: u16 =  0o004;	// Global Read
pub const S_IWOTH: u16 =  0o002;	// Global Write
pub const S_IXOTH: u16 =  0o001;	// Global Execute

pub const EXT4_INDEX_FL: u16 = 0x1000;	// i_flags: Directory uses a hashed btree

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
//def_from_slice!{ GroupDesc }
impl_fmt! {
	Debug(self, f) for GroupDesc {
		write!(f, "GroupDesc {{ addrs: (block_bm: {}, inode_bm: {}, inodes: {}), counts: (free_blk: {}, free_inodes: {}, used_dirs: {}) }}",
			self.bg_block_bitmap, self.bg_inode_bitmap, self.bg_inode_table,
			self.bg_free_blocks_count, self.bg_free_inodes_count, self.bg_used_dirs_count
			)
	}
}



#[repr(C)]
pub struct DirEnt
{
	/// Inode number
	pub d_inode: u32,
	/// Directory entry length
	pub d_rec_len: u16,
	/// Name Length
	pub d_name_len: u8,
	/// File Type (Duplicate of ext2_inode_s.i_mode)
	/// NOTE: This is only populated if FEAT_INCOMPAT_FILETYPE is present
	pub d_type: u8,
	/// Actual file name
	pub d_name: [u8],	// EXT2_NAME_LEN+1
}
pub const DIRENT_MIN_SIZE: usize = 8;

//pod_impls!{ DirEnt }

impl DirEnt
{
	pub fn new_raw(buf: &[u32], name_len: usize) -> *const DirEnt
	{
		// SAFE: Returns a raw pointer, alignment is valid though
		unsafe {
			::core::slice::from_raw_parts(buf.as_ptr() as *const u8, name_len) as *const [u8] as *const DirEnt
		}
	}
	pub fn new(buf: &[u32]) -> Option<&DirEnt>
	{
		assert!(buf.len() >= 8/4);
		// SAFE: 0 name length is valid
		let rv0: &DirEnt = unsafe { &*Self::new_raw(buf, 0) };

		let rec_len = rv0.d_rec_len as usize;
		let name_len = rv0.d_name_len as usize;

		if rec_len > buf.len() * 4 {
			log_warning!("Consistency error: Record too long {} > {}", rec_len, buf.len()*4);
			None
		}
		else if name_len + 8 > rec_len {
			log_warning!("Consistency error: Record too long {}+8 > {} (name exceeds space)", name_len+8, rec_len);
			None
		}
		else {
			// SAFE: Name length has just been checked
			let rv_n = unsafe { &*Self::new_raw(buf, name_len) };
			Some(rv_n)
		}
	}

	pub fn new_mut(buf: &mut [u32]) -> Option<&mut DirEnt>
	{
		match Self::new(buf)
		{
		// SAFE: &mut in, &mut out
		Some(v) => Some( unsafe { &mut *(v as *const _ as *mut _) } ),
		None => None,
		}
	}


	/// Returns the number of 32-bit integers this entry takes up
	pub fn u32_len(&self) -> usize {
		(self.d_rec_len as usize + 3) / 4
	}
}

impl_fmt! {
	Debug(self,f) for DirEnt {
		write!(f, "DirEnt {{ d_inode: {}, d_rec_len: {}, d_type: {}, d_name: {:?} }}",
			self.d_inode, self.d_rec_len, self.d_type, ::kernel::lib::byte_str::ByteStr::new(&self.d_name)
			)
	}
}
