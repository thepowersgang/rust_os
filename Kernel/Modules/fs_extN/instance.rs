// "Tifflin" Kernel - ext2/3/4 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_extN/instance.rs
//! Filesystem instance (representing a mounted filesystem)
use kernel::prelude::*;
use kernel::vfs::{self, node};
use kernel::metadevs::storage::VolumeHandle;
use kernel::lib::mem::aref::{ArefInner,ArefBorrow};

pub struct Instance(ArefInner<InstanceInner>);
pub type InstancePtr = ArefBorrow<InstanceInner>;

pub struct InstanceInner
{
	pub vol: ::buffered_volume::BufferedVolume,
	superblock: ::ondisk::Superblock,
	pub fs_block_size: usize,

	group_descriptors: Vec<::ondisk::GroupDesc>,
}

pub enum FeatureState
{
	AllOk,
	Reduced(u32),
	ReadOnly(u32),
	Incompatible(u32),
}

impl Instance
{
	pub fn check_features(vol_name: &str, sb: &::ondisk::Superblock) -> FeatureState
	{
		// Legacy (no feature flags)
		if sb.data.s_rev_level == 0 {
			FeatureState::AllOk
		}
		else {
			let unsupported_req = sb.ext.s_feature_incompat  & !::SUPPORTED_REQ_FEATURES;
			let unsupported_rdo = sb.ext.s_feature_ro_compat & !::SUPPORTED_RDO_FEATURES;
			let unsupported_opt = sb.ext.s_feature_compat    & !::SUPPORTED_OPT_FEATURES;
			if unsupported_req != 0 {
				// Can't even read correctly
				log_warning!("Volume `{}` uses incompatible required features (unsupported bits {:#x})", vol_name, unsupported_req);
				FeatureState::Incompatible( unsupported_req )
			}
			else if unsupported_rdo != 0 {
				// Read-only
				log_warning!("Volume `{}` uses incompatible read-write features (unsupported bits {:#x})", vol_name, unsupported_rdo);
				FeatureState::ReadOnly( unsupported_rdo )
			}
			else if unsupported_opt != 0 {
				// Can read and write, but may confuse other systems
				log_warning!("Volume `{}` uses incompatible optional features (unsupported bits {:#x})", vol_name, unsupported_opt);
				FeatureState::Reduced( unsupported_opt )
			}
			else {
				// Fully supported
				FeatureState::AllOk
			}
		}
	}

	pub fn new_boxed(vol: VolumeHandle) -> vfs::Result<Box<Instance>>
	{
		let vol_bs = vol.block_size();

		// The superblock exists at offset 1024 in the volume, no matter the on-disk block size
		let superblock_idx = (1024 / vol_bs) as u64;
		let superblock_ofs = (1024 % vol_bs) as usize;

		let (superblock, first_block) = {
			let mut first_block: Vec<u32> = vec![0; ::core::cmp::max(1024, vol_bs)/4];
			try!(vol.read_blocks(superblock_idx, ::kernel::lib::as_byte_slice_mut(&mut first_block[..])));
			assert!(superblock_ofs % 4 == 0);
			(
				*::ondisk::Superblock::from_slice(&first_block[superblock_ofs/4 ..][..1024/4]),
				first_block,
				)
			};


		if superblock.data.s_magic != 0xEF53 {
			return Err(vfs::Error::TypeMismatch);
		}
		match Self::check_features(vol.name(), &superblock)
		{
		FeatureState::Incompatible(_) => return Err(vfs::Error::TypeMismatch),
		_ => {},
		}

		// - Limit block size to 1MB each
		if superblock.data.s_log_block_size > 10 {
			return Err(vfs::Error::Unknown("extN block size out of range"));
		}

		let fs_block_size = 1024 << superblock.data.s_log_block_size as usize;
		if fs_block_size % vol_bs != 0 {
			log_warning!("ExtN TODO: Handle filesystem block size smaller than disk block size?");
			return Err(vfs::Error::ConsistencyError);
		}
		let num_groups = ::kernel::lib::num::div_up(superblock.data.s_blocks_count, superblock.data.s_blocks_per_group);

		// Read group descriptor table
		// - This always resides immediately after the superblock
		let group_descs = {
			use kernel::lib::{as_byte_slice_mut,as_byte_slice};

			let groups_per_vol_block = vol_bs / ::core::mem::size_of::<::ondisk::GroupDesc>();

			let mut gds: Vec<::ondisk::GroupDesc> = vec![Default::default(); num_groups as usize];

			let (n_skip, mut block) = if vol_bs % (2*1024) == 0 {
					// Volume block size is larger than the superblock
					// - This means that at least 2048 bytes of the group descriptors are in the same block as the superblock
					let n_shared = (vol_bs - 2*1024) / ::core::mem::size_of::<::ondisk::GroupDesc>();

					let src = as_byte_slice(&first_block[2*1024/4..]);
					if n_shared >= gds.len()
					{
						as_byte_slice_mut(&mut gds[..]).clone_from_slice( src );
						(gds.len(), 1)
					}
					else
					{
						as_byte_slice_mut(&mut gds[..n_shared]).clone_from_slice( src );
						(n_shared, 1)
					}
				}
				else {
					// Volume BS <= superblock
					(0, (2*1024 / vol_bs) as u64)
				};

			let rem_count = gds.len() - n_skip;
			let tail_count = rem_count % groups_per_vol_block;
			let body_count = rem_count - tail_count;
			log_trace!("n_skip={}, block={}, rem_count={},  tail_count={}, body_count={}",
				n_skip, block, rem_count,  tail_count, body_count);

			if body_count > 0 
			{
				try!(vol.read_blocks(block, as_byte_slice_mut(&mut gds[n_skip .. ][ .. body_count])));
				block += (body_count / groups_per_vol_block) as u64;
			}

			if tail_count > 0
			{
				// Read a single volume block into a buffer, then populate from that
				let mut buf: Vec<u8> = vec![0; vol_bs];
				try!(vol.read_blocks(block, &mut buf));
				as_byte_slice_mut(&mut gds[n_skip + body_count ..]).clone_from_slice( &buf );
			}

			gds
			};


		for (i, gd) in group_descs.iter().enumerate()
		{
			log_debug!("{}: Group #{}: {:?}", vol.name(), i, gd);
		}

		let inner = InstanceInner {
			fs_block_size: fs_block_size,
			superblock: superblock,
			group_descriptors: group_descs,
			vol: ::buffered_volume::BufferedVolume::new(vol),
			};

		// SAFE: Boxed instantly
		unsafe {
			Ok(Box::new(Instance(ArefInner::new( inner ))))
		}
	}
}

impl vfs::mount::Filesystem for Instance
{
	fn root_inode(&self) -> node::InodeId {
		// ext* uses inode 2 as the root
		2
	}
	fn get_node_by_inode(&self, id: node::InodeId) -> Option<node::Node> {
		log_trace!("get_node_by_inode(id={})", id);
		let inode = match ::inodes::Inode::from_id(self.0.borrow(), id as u32)
			{
			Ok(v) => v,
			Err(e) => {
				log_error!("get_node_by_inode - IO error {:?}", e);
				return None;
				},
			};
		match inode.i_mode_fmt()
		{
		0 => None,
		::ondisk::S_IFREG => {
			Some( node::Node::File( Box::new( ::file::File::new(inode) )  ) )
			},
		::ondisk::S_IFDIR => {
			Some( node::Node::Dir( Box::new( ::dir::Dir::new(inode) )  ) )
			},
		v @ _ => {
			log_warning!("TODO: Handle node format {} in extN get_node_by_inode", v >> 12);
			None
			},
		}
	}
}

impl InstanceInner
{
	/// Obtain a block (possibly cached)
	pub fn get_block(&self, block: u32) -> vfs::node::Result<Box<[u32]>>
	{
		log_trace!("get_block({})", block);
		let mut rv = vec![0u32; self.fs_block_size / 4].into_boxed_slice();
		try!( self.read_blocks( block, ::kernel::lib::as_byte_slice_mut(&mut rv[..]) ) );
		Ok(rv)
	}
	/// Read a sequence of blocks into a user-provided buffer
	pub fn read_blocks(&self, first_block: u32, data: &mut [u8]) -> vfs::node::Result<()>
	{
		try!( self.vol.read_blocks( first_block as u64 * self.vol_blocks_per_fs_block(), data) );
		Ok( () )
	}
}

/// Inode lookup and save
impl InstanceInner
{
	/// Returns (volblock, byte_ofs)
	fn get_inode_pos(&self, inode_num: u32) -> (u64, usize) {
		assert!(inode_num != 0);
		let inode_num = inode_num - 1;

		let (group, ofs) = ( inode_num / self.s_inodes_per_group(), inode_num % self.s_inodes_per_group() );
		let base_blk_id = self.group_descriptors[group as usize].bg_inode_table as u64 * self.vol_blocks_per_fs_block();
		let ofs_bytes = (ofs as usize) * self.s_inode_size();
		let (sub_blk_id, sub_blk_ofs) = (ofs_bytes / self.vol.block_size(), ofs_bytes % self.vol.block_size());

		(base_blk_id + sub_blk_id as u64,  sub_blk_ofs as usize)
	}

	/// Read an inode descriptor from the disk
	pub fn read_inode(&self, inode_num: u32) -> vfs::Result< ::ondisk::Inode >
	{
		let (vol_block, blk_ofs) = self.get_inode_pos(inode_num);
		log_trace!("read_inode({}) - vol_block={}, blk_ofs={}", inode_num, vol_block, blk_ofs);

		let mut rv = ::ondisk::Inode::default();
		{
			// NOTE: Unused fields in the inode are zero
			let slice = &mut ::kernel::lib::as_byte_slice_mut(&mut rv)[.. self.s_inode_size()];
			try!( self.vol.read_subblock_single(vol_block, blk_ofs, slice) );
		}
		log_trace!("- rv={:?}", rv);
		Ok( rv )
	}
	/// Write an inode descriptor back to the disk
	pub fn write_inode(&self, inode_num: u32, inode_data: ::ondisk::Inode) -> vfs::Result< () >
	{
		let (vol_block, blk_ofs) = self.get_inode_pos(inode_num);
		
		let slice = &::kernel::lib::as_byte_slice(&inode_data)[.. self.s_inode_size()];
		try!( self.vol.write_subblock_single(vol_block, blk_ofs, slice) );

		Ok( () )
	}
}

/// Superblock parameters
impl InstanceInner
{
	fn s_inodes_per_group(&self) -> u32 {
		self.superblock.data.s_inodes_per_group
	}

	fn vol_blocks_per_fs_block(&self) -> u64 {
		(self.fs_block_size / self.vol.block_size()) as u64
	}

	fn s_inode_size(&self) -> usize {
		if self.superblock.data.s_rev_level > 0 {
			self.superblock.ext.s_inode_size as usize
		}
		else {
			128
		}
	}
}


