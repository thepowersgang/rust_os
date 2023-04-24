// "Tifflin" Kernel - ext2/3/4 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_extN/instance.rs
//! Filesystem instance (representing a mounted filesystem)
use kernel::prelude::*;
use ::vfs::{self, node};
use kernel::metadevs::storage::VolumeHandle;
use kernel::lib::mem::aref::{ArefInner,ArefBorrow};

pub struct Instance(ArefInner<InstanceInner>);
pub type InstancePtr = ArefBorrow<InstanceInner>;

pub struct InstanceInner
{
	is_readonly: bool,
	pub vol: ::block_cache::CachedVolume,
	superblock: ::kernel::sync::RwLock<crate::ondisk::Superblock>,
	pub fs_block_size: usize,

	mount_handle: vfs::mount::SelfHandle,
	group_descriptors: ::kernel::sync::RwLock< Vec<::ondisk::GroupDesc> >,
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

	pub fn new_boxed(vol: VolumeHandle, mount_handle: vfs::mount::SelfHandle) -> vfs::Result<Box<Instance>>
	{
		let vol_bs = vol.block_size();

		// The superblock exists at offset 1024 in the volume, no matter the on-disk block size
		let superblock_idx = (1024 / vol_bs) as u64;
		let superblock_ofs = (1024 % vol_bs) as usize;

		let (superblock, first_block) = {
			let mut first_block: Vec<u8> = vec![0; ::core::cmp::max(1024, vol_bs)];
			::kernel::futures::block_on(vol.read_blocks(superblock_idx, &mut first_block[..]))?;
			assert!(superblock_ofs % 4 == 0);
			(
				::ondisk::Superblock::from_slice(&first_block[superblock_ofs ..][..1024]),
				first_block,
				)
			};

		if superblock.data.s_magic != 0xEF53 {
			return Err(vfs::Error::TypeMismatch);
		}
		log_debug!("superblock = {:x?}", superblock);

		let is_readonly = match Self::check_features(vol.name(), &superblock)
			{
			FeatureState::Incompatible(_) => return Err(vfs::Error::TypeMismatch),
			FeatureState::ReadOnly(_) => true,
			_ => false,
			};

		// Limit filesystem block size to 1MB each, as a sanity check
		if superblock.data.s_log_block_size > 10 {
			return Err(vfs::Error::Unknown("extN block size out of range"));
		}

		let fs_block_size = 1024 << superblock.data.s_log_block_size as usize;
		if fs_block_size % vol_bs != 0 {
			log_warning!("ExtN TODO: Handle filesystem block size smaller than disk block size?");
			return Err(vfs::Error::InconsistentFilesystem);
		}
		let num_groups = ::kernel::lib::num::div_up(superblock.data.s_blocks_count, superblock.data.s_blocks_per_group);

		// Read group descriptor table
		// - This resides in the first FS block after the superblock
		let group_descs = {
			const GROUP_DESC_SIZE: usize = ::core::mem::size_of::<::ondisk::GroupDesc>();
			if GROUP_DESC_SIZE != superblock.s_group_desc_size() {
				return Err(vfs::Error::Unknown("Superblock size mismatch vs expected"));
			}

			let groups_per_vol_block = vol_bs / GROUP_DESC_SIZE;
			// Group descriptors are in the first filesystem block after the superblock
			// - So either immediately right after the superblock, or the second block (whichever is larger)
			let byte_offset = usize::max(2*1024, fs_block_size);
			log_trace!("Group Descs: {} groups @ byte {}, {} per volume block (vol_bs={})",
				num_groups, byte_offset, groups_per_vol_block, vol_bs);

			let mut gds: Vec<::ondisk::GroupDesc> = (0..num_groups).map(|_| Default::default()).collect();

			// The superblock is 1024 bytes at offset 1024
			// - If the volume block size is 2K or larger, then there are some group descriptors in the first block
			let (n_skip, mut vol_block) = if vol_bs > byte_offset {
					// Volume block size is larger than the offset
					// - This means that at least 2048 bytes of the group descriptors are in the same block as the superblock
					let n_shared = (vol_bs - byte_offset) / GROUP_DESC_SIZE;

					let mut src = &first_block[byte_offset..];
					let count = ::core::cmp::min(n_shared, gds.len());
					assert_eq!(src.len(), count * GROUP_DESC_SIZE);
					for s in &mut gds[..count] {
						*s = ::kernel::lib::byteorder::EncodedLE::decode(&mut src).unwrap();
						log_debug!("GROUP DESC: {:?}", s);
					}
					(count, 1)
				}
				else {
					// Volume BS <= superblock
					// - Offset of byte 2048 in the disk
					(0, (byte_offset / vol_bs) as u64)
				};

			// Determine how many descriptors are in the subsequent volume blocks
			let rem_count = gds.len() - n_skip;
			let tail_count = rem_count % groups_per_vol_block;
			let body_count = rem_count - tail_count;
			log_trace!("vol_block={} n_skip={} => rem_count={} (tail_count={}, body_count={})",
				vol_block, n_skip, rem_count,  tail_count, body_count);

			let mut buf: Vec<u8> = vec![0; vol_bs];
			if body_count > 0
			{
				for gds in gds[n_skip..][..body_count].chunks_mut(groups_per_vol_block) {
					::kernel::futures::block_on(vol.read_blocks(vol_block, &mut buf))?;
					let mut src = &buf[..];
					for s in gds {
						*s = ::kernel::lib::byteorder::EncodedLE::decode(&mut src).unwrap();
						log_debug!("GROUP DESC: {:?}", s);
					}
					vol_block += 1;
				}
			}

			if tail_count > 0
			{
				let ofs = n_skip + body_count;
				// Read a single volume block into a buffer, then populate from that
				::kernel::futures::block_on(vol.read_blocks(vol_block, &mut buf))?;
				let n_bytes = (gds.len() - ofs) * GROUP_DESC_SIZE;
				let mut src = &buf[..n_bytes];
				for s in &mut gds[ofs..] {
					*s = ::kernel::lib::byteorder::EncodedLE::decode(&mut src).unwrap();
					log_debug!("GROUP DESC: {:?}", s);
				}
			}

			gds
			};


		for (i, gd) in group_descs.iter().enumerate()
		{
			log_debug!("{}: Group #{}: {:?}", vol.name(), i, gd);
		}

		let inner = InstanceInner {
			is_readonly: is_readonly,
			fs_block_size: fs_block_size,
			superblock: ::kernel::sync::RwLock::new(superblock),
			group_descriptors: ::kernel::sync::RwLock::new(group_descs),
			mount_handle: mount_handle,
			vol: ::block_cache::CachedVolume::new(vol),
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
		match { let v = inode.lock_read().i_mode_fmt(); v }
		{
		0 => {
			log_debug!("get_node_by_inode({}): i_mode_fmt=0", id);
			None
			},
		::ondisk::S_IFREG => Some( node::Node::File( Box::new( ::file::File::new(inode) )  ) ),
		::ondisk::S_IFDIR => Some( node::Node::Dir( Box::new( ::dir::Dir::new(inode) )  ) ),
		v @ _ => {
			log_warning!("TODO: Handle node format {} in extN get_node_by_inode", v >> 12);
			None
			},
		}
	}
}

impl InstanceInner
{
	pub fn is_readonly(&self) -> bool
	{
		self.is_readonly
	}
}

/// Structure representing a view into a BlockCache entry
pub struct Block<'a>(::block_cache::BlockHandleRead<'a>, u16,u16);
impl<'a> ::core::ops::Deref for Block<'a>
{
	type Target = [u32];
	fn deref(&self) -> &[u32] {
		let &Block(ref handle, ofs, size) = self;
		// SAFE: Alignment should be good (but is checked anyway)
		unsafe {
			assert!(ofs as usize + size as usize <= handle.data().len());
			assert!(ofs % 4 == 0);
			assert!(&handle.data()[0] as *const _ as usize % 4 == 0);
			::core::slice::from_raw_parts(&handle.data()[ofs as usize] as *const u8 as *const u32, (size / 4) as usize)
		}
	}
}

impl InstanceInner
{
	/// Obtain a block (using the block cache)
	pub fn get_block(&self, block: u32) -> vfs::node::Result<Block>
	{
		if self.fs_block_size > ::kernel::PAGE_SIZE {
			// TODO: To handle extN blocks larger than the system's page size, we'd need to start packing multiple cache handles into
			//       the `Block` structure
			todo!("Handle extN block sizes > PAGE_SIZE - {} > {}", self.fs_block_size, ::kernel::PAGE_SIZE);
		}
		log_trace!("get_block({})", block);
		let sector = block as u64 * self.vol_blocks_per_fs_block();

		let ch = ::kernel::futures::block_on(self.vol.get_block(sector))?;
		let ofs = (sector - ch.index()) as usize * self.vol.block_size();
		Ok( Block(ch, ofs as u16, self.fs_block_size as u16) )
	}

	/// Edit a block in the cache using the provided closure
	pub fn edit_block<F,R>(&self, block: u32, f: F) -> vfs::node::Result<R>
	where
		F: FnOnce(&mut [u8]) -> vfs::node::Result<R>
	{
		if self.fs_block_size > ::kernel::PAGE_SIZE {
			// TODO: To handle extN blocks larger than the system's page size, we'd need to start packing multiple cache handles into
			//       the `Block` structure
			todo!("Handle extN block sizes > PAGE_SIZE - {} > {}", self.fs_block_size, ::kernel::PAGE_SIZE);
		}
		log_trace!("get_block({})", block);
		let sector = block as u64 * self.vol_blocks_per_fs_block();

		::kernel::futures::block_on(self.vol.edit(sector, self.vol_blocks_per_fs_block() as usize, |data| {
			f(data)
			}))?
	}

	pub fn read_blocks_inner<F,R>(&self, first_block: u32, ofs: usize, len: usize, f: F) -> vfs::node::Result<R>
	where
		F: FnOnce(&[u8]) -> vfs::node::Result<R>
	{
		todo!("");
	}
	/// Obtain a block (uncached)
	///
	/// This is the more expensive version of `get_block`, which doesn't directly touch the block cache.
	/// It's used to handle partial file reads (which should be cached by higher layers)
	pub fn get_block_uncached(&self, block: u32) -> vfs::node::Result<Box<[u8]>>
	{
		log_trace!("get_block_uncached({})", block);
		let mut rv = vec![0; self.fs_block_size].into_boxed_slice();
		self.read_blocks( block, &mut rv[..] )?;
		Ok(rv)
	}

	/// Read a sequence of blocks into a user-provided buffer
	pub fn read_blocks(&self, first_block: u32, data: &mut [u8]) -> vfs::node::Result<()>
	{
		::kernel::futures::block_on( self.vol.read_blocks_uncached( first_block as u64 * self.vol_blocks_per_fs_block(), data) )?;
		Ok( () )
	}

	/// Write a sequence of blocks from a user-provided buffer
	pub fn write_blocks(&self, first_block: u32, data: &[u8]) -> vfs::node::Result<()>
	{
		// TODO: Requires maybe interfacing with the cache used by get_block?
		::kernel::futures::block_on( self.vol.write_blocks_uncached( first_block as u64 * self.vol_blocks_per_fs_block(), data) )?;
		Ok( () )
	}
}

impl InstanceInner
{
	fn get_block_grp_id(&self, block_idx: u32) -> (u32, u32) {
		let s_blocks_per_group = self.superblock.read().data.s_blocks_per_group;
		(block_idx / s_blocks_per_group, block_idx % s_blocks_per_group)
	}
	/// Allocate a new data block
	pub fn allocate_data_block(&self, inode_num: u32, prev_block: u32) -> vfs::node::Result<u32> {
		log_debug!("allocate_data_block(inode_num=I{}, prev_block=B{})", inode_num, prev_block);
		let has_blocks = self.edit_superblock(|sb| {
			if sb.has_feature_incompat(crate::ondisk::FEAT_INCOMPAT_64BIT) {
				if sb.data.s_free_blocks_count == 0 && sb.ext.s_free_blocks_count_hi == 0 {
					false
				}
				else {
					let (new, overflow) = sb.data.s_free_blocks_count.overflowing_sub(1);
					if overflow {
						sb.ext.s_free_blocks_count_hi -= 1;
					}
					sb.data.s_free_blocks_count = new;
					true
				}
			}
			else {
				if sb.data.s_free_blocks_count == 0 {
					false
				}
				else {
					sb.data.s_free_blocks_count -= 1;
					true
				}
			}
			})?;
		if !has_blocks {
			return Err(vfs::Error::OutOfSpace);
		}
		if prev_block != 0 {
			let (block_bg, block_subidx) = self.get_block_grp_id(prev_block);
			let inode_bg = self.get_inode_grp_id(inode_num).0;
			// 1. Check witin the same BG (telling it the previous block, so it can pick one near that)
			if let Some(rv) = self.allocate_block_in_group(block_bg, prev_block)? {
				return Ok(rv);
			}
			// 2. If the inode BG is different, then check in the inode's BG
			if block_bg != inode_bg {
				if let Some(rv) = self.allocate_block_in_group(inode_bg, 0)? {
					return Ok(rv);
				}
			}
		}
		else {
			let inode_bg = self.get_inode_grp_id(inode_num).0;
			if let Some(rv) = self.allocate_block_in_group(inode_bg, 0)? {
				return Ok(rv);
			}
		}
		// Fallback: Otherwise, first available within the inodes BG
		// Or, from a random block group
		todo!("allocate_data_block(inode={}, prev_block={})", inode_num, prev_block);
	}

	fn allocate_block_in_group(&self, group: u32, prev_block: u32) -> vfs::node::Result<Option<u32>> {
		let first_bmp_block = self.group_descriptors.read()[group as usize].bg_block_bitmap;
		// Prefer allocating within a few blocks of the previous (ideally right after) - if non zero
		if prev_block != 0 {
			let next_block = prev_block + 1;
			let (block_bg, block_subidx) = self.get_block_grp_id(next_block);
			if block_bg == group {
				// Edit the bitmap, check if this bit is clear
				let bmp_mask = 1 << (block_subidx % 8);
				let bmp_byte = block_subidx / 8;
				let bmp_block = bmp_byte / self.fs_block_size as u32;
				let bmp_byte = bmp_byte % self.fs_block_size as u32;
				// If it is, then set it and decrement the (non-zero) free block count
				if self.edit_block(first_bmp_block + bmp_block, |blk_data| {
					if blk_data[bmp_byte as usize] & bmp_mask == 0 {
						blk_data[bmp_byte as usize] |= bmp_mask;
						Ok(true)
					}
					else {
						Ok(false)
					}
					})? {
					if !self.edit_block_group_header(block_bg, |bg| if bg.bg_free_blocks_count == 0 { false } else { bg.bg_free_blocks_count -= 1; true })? {
						return Err(vfs::Error::InconsistentFilesystem);
					}
					log_debug!("allocate_block_in_group(): return next B{}", next_block);
					return Ok(Some(next_block));
				}
			}
		}

		// Decrement the block count, and then find an entry
		if !self.edit_block_group_header(group, |bg| if bg.bg_free_inodes_count == 0 { false } else { bg.bg_free_blocks_count -= 1; true })?
		{
			return Ok(None);
		}

		// Iterate the bitmap
		let blocks_per_bmpblock = self.fs_block_size * 8;
		let s_blocks_per_group = self.superblock.read().data.s_blocks_per_group;
		for base in (0 .. s_blocks_per_group).step_by(blocks_per_bmpblock) {
			// Number of inodes in this bitmap block (might be fewer, if the group size is small)
			let n_blocks = (s_blocks_per_group - base).min(blocks_per_bmpblock as u32);
			let n_bytes = ::kernel::lib::num::div_up(n_blocks, 8) as usize;
			let rv = self.edit_block(first_bmp_block + base / blocks_per_bmpblock as u32, |blk_data| {
				Ok(match blk_data[..n_bytes].iter().position(|&v| v != !0)
				{
				None => None,
				Some(p) => {
					let bit = blk_data[p].trailing_ones();
					blk_data[p] |= 1 << bit;
					Some(p as u32 * 8 + bit)
					},
				})
				})?;
			if let Some(rel_block_id) = rv {
				// Check for the edge case where the returned ID is outside the expected bounds
				if rel_block_id >= n_blocks {
					break
				}
				let rv = group * s_blocks_per_group + base + rel_block_id;
				log_debug!("allocate_block_in_bg({}) Allocate B{}", group, rv);
				return Ok(Some(rv));
			}
		}
		log_error!("allocate_block_in_group: Descriptor said that there were free blocks, but bitmap was full.");
		Err(vfs::Error::InconsistentFilesystem)
	}
}

impl InstanceInner
{
	fn edit_superblock<R>(&self, cb: impl FnOnce(&mut crate::ondisk::Superblock)->R) -> vfs::node::Result<R> {
		let mut lh = self.superblock.write();
		let rv = cb(&mut lh);
		if self.vol.block_size() > 1024 {
			::kernel::futures::block_on(self.vol.edit(0, 1, |data| {
				let data = &mut data[1024..][..1024];
				lh.write_to_slice(data);
				}))?;
		}
		else {
			::kernel::futures::block_on(self.vol.edit(1024 / self.vol.block_size() as u64, 1024 / self.vol.block_size(), |data| {
				lh.write_to_slice(data);
				}))?;
		}
		Ok(rv)
	}
	fn edit_block_group_header<R>(&self, idx: u32, cb: impl FnOnce(&mut crate::ondisk::GroupDesc)->R) -> vfs::node::Result<R> {
		let mut lh = self.group_descriptors.write();
		let rv = cb(&mut lh[idx as usize]);
		let ofs = 1024 + idx as usize * ::core::mem::size_of::<::ondisk::GroupDesc>();
		::kernel::futures::block_on(self.vol.edit( (ofs / self.vol.block_size()) as u64, 1, |data| {
			let buf = &mut data[ofs % self.vol.block_size()..][..::core::mem::size_of::<::ondisk::GroupDesc>()];
			lh[idx as usize].write_to_slice(buf);
			}))?;
		Ok(rv)
	}
}

/// Inode lookup and save
impl InstanceInner
{
	/// Returns (grp_idx, inner_idx)
	fn get_inode_grp_id(&self, inode_num: u32) -> (u32, u32) {
		assert!(inode_num != 0);
		let inode_num = inode_num - 1;

		let s_inodes_per_group = self.superblock.read().s_inodes_per_group();
		( inode_num / s_inodes_per_group, inode_num % s_inodes_per_group )
	}
	/// Returns (volblock, byte_ofs)
	fn get_inode_pos(&self, inode_num: u32) -> (u64, usize) {
		let (group, ofs) = self.get_inode_grp_id(inode_num);

		let base_blk_id = self.group_descriptors.read()[group as usize].bg_inode_table as u64 * self.vol_blocks_per_fs_block();
		assert!(base_blk_id != 0);
		let ofs_bytes = (ofs as usize) * self.superblock.read().s_inode_size();
		let (sub_blk_id, sub_blk_ofs) = (ofs_bytes / self.vol.block_size(), ofs_bytes % self.vol.block_size());

		(base_blk_id + sub_blk_id as u64, sub_blk_ofs as usize)
	}

	/// Perform an operation with a temporary handle to an inode
	pub fn with_inode<F,R>(&self, inode_num: u32, fcn: F) -> vfs::node::Result<R>
	where
		F: FnOnce(&::inodes::Inode) -> vfs::node::Result<R>
	{
		// TODO: Hook into the VFS's node cache somehow (we'd need to know our mount ID) and
		//       obtain a reference to a cached inode.
		// - This prevents us from having to maintain our own node cache

		let node = try!(self.mount_handle.get_node(inode_num as vfs::node::InodeId));
		match node.get_node_any().downcast_ref()
		{
		Some(our_in) => fcn(our_in),
		None => Err(vfs::Error::Unknown("BUG: Node wasn't an extN inode")),
		}
	}

	/// Allocate a new inode number, possibly in the same block group as `parent_inode_num`.
	pub fn allocate_inode(&self, parent_inode_num: u32, nodetype: vfs::node::NodeType) -> vfs::node::Result< u32 >
	{
		// TODO: Update the superblock
		let has_inodes = self.edit_superblock(|sb| {
			if sb.data.s_free_inodes_count == 0 {
				false
			}
			else {
				sb.data.s_free_inodes_count -= 1;
				true
			}
			})?;
		if !has_inodes {
			return Err(vfs::Error::OutOfSpace);
		}

		assert!(parent_inode_num != 0);	// Has to be a parent - root exists
		let (grp, _idx) = self.get_inode_grp_id(parent_inode_num);

		let rv = if let Some(rv) = self.allocate_inode_in_bg(grp, nodetype)? {
				rv
			}
			else {
				todo!("InstanceInner::allocate_inode - Search for any BG");
			};

		self.write_inode(rv, &crate::ondisk::Inode {
			i_mode: match nodetype
				{
				vfs::node::NodeType::File => ::ondisk::S_IFREG,
				_ => todo!(""),
				},
			..Default::default()
			})?;

		Ok(rv)
	}
	fn allocate_inode_in_bg(&self, grp: u32, _nodetype: vfs::node::NodeType) -> vfs::node::Result< Option<u32> > {
		// NOTE: Check with read-only first, and only read-modify-write if the read-only check passed
		if self.group_descriptors.read()[grp as usize].bg_free_inodes_count == 0 {
			return Ok(None);
		}
		let Some(first_bmp_block) = self.edit_block_group_header(grp, |gd|
			if gd.bg_free_inodes_count == 0 {
				// This should only be hit if there is a race
				None
			}
			else {
				gd.bg_free_inodes_count -= 1;
				Some(gd.bg_inode_bitmap)
			})? else {
			return Ok(None);
		};

		// Start the bitmap check
		let inodes_per_block = self.fs_block_size * 8;
		let s_inodes_per_group = self.superblock.read().s_inodes_per_group();
		for base in (0 .. s_inodes_per_group).step_by(inodes_per_block) {
			// Number of inodes in this bitmap block (might be fewer, if the group size is small)
			let n_inodes = (s_inodes_per_group - base).min(inodes_per_block as u32);
			let n_bytes = ::kernel::lib::num::div_up(n_inodes, 8) as usize;
			let rv = self.edit_block(first_bmp_block + base / inodes_per_block as u32, |blk_data| {
				Ok(match blk_data[..n_bytes].iter().position(|&v| v != !0)
				{
				None => None,
				Some(p) => {
					let bit = blk_data[p].trailing_ones();
					blk_data[p] |= 1 << bit;
					Some(p as u32 * 8 + bit)
					},
				})
				})?;
			if let Some(rel_inode_id) = rv {
				// Check for the edge case where the returned ID is outside the expected bounds (could happen if s_inodes_per_group is
				// not a multiple of 32)
				if rel_inode_id >= n_inodes {
					break
				}
				let rv = 1 + grp * s_inodes_per_group + base + rel_inode_id;
				log_debug!("allocate_inode_in_bg({}) Allocate I{}", grp, rv);
				assert!(rv > 2);
				return Ok(Some(rv));
			}
		}
		log_error!("allocate_inode_in_bg: Descriptor said that there were free inodes, but bitmap was full.");
		Err(vfs::Error::InconsistentFilesystem)
	}

	/// Read an inode descriptor from the disk
	pub fn read_inode(&self, inode_num: u32) -> vfs::Result< ::ondisk::Inode >
	{
		let (vol_block, blk_ofs) = self.get_inode_pos(inode_num);
		log_trace!("read_inode({}) - vol_block={}, blk_ofs={}", inode_num, vol_block, blk_ofs);

		let rv = {
			let s_inode_size = self.superblock.read().s_inode_size();
			// NOTE: Unused fields in the inode are zero
			let mut buf = vec![0; ::core::mem::size_of::<crate::ondisk::Inode>()];
			let slice = if buf.len() > s_inode_size {
					&mut buf[..s_inode_size]
				} else {
					&mut buf
				};
			::kernel::futures::block_on( self.vol.read_inner(vol_block, blk_ofs, slice) )?;
			::ondisk::Inode::from_slice(&buf[..])
		};
		log_trace!("- rv={:?}", rv);
		Ok( rv )
	}
	/// Write an inode descriptor back to the disk
	pub fn write_inode(&self, inode_num: u32, inode_data: &::ondisk::Inode) -> vfs::Result< () >
	{
		let (vol_block, blk_ofs) = self.get_inode_pos(inode_num);

		let s_inode_size = self.superblock.read().s_inode_size();
		::kernel::futures::block_on(self.vol.edit(vol_block, 1, |data| {
			let mut slice = &mut data[blk_ofs..][..s_inode_size];
			let _ = ::kernel::lib::byteorder::EncodedLE::encode(inode_data, &mut slice);
			}))?;

		Ok( () )
	}
}

/// Superblock parameters
impl InstanceInner
{
	fn vol_blocks_per_fs_block(&self) -> u64 {
		(self.fs_block_size / self.vol.block_size()) as u64
	}

	pub fn has_feature_incompat(&self, feat: u32) -> bool {
		self.superblock.read().has_feature_incompat(feat)
	}
	pub fn has_feature_ro_compat(&self, feat: u32) -> bool {
		self.superblock.read().has_feature_ro_compat(feat)
	}
}


