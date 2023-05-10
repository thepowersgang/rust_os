/*
 */
use ::kernel::prelude::*;
use ::kernel::metadevs::storage::VolumeHandle;
use ::kernel::lib::mem::aref;
use crate::ondisk;
use crate::MftEntryIdx;

/// A wrapper around the instance, owned by the VFS layer
pub struct InstanceWrapper(aref::ArefInner<Instance>);
/// 
pub type InstanceRef = aref::ArefBorrow<Instance>;

pub struct Instance
{
	vol: ::block_cache::CachedVolume,
	mount_handle: ::vfs::mount::SelfHandle,
	cluster_size_blocks: usize,
	mft_record_size: usize,
	mft_data_attr: Option<ondisk::AttrHandle>,
	bs: ondisk::Bootsector,
}

impl Instance
{
	pub fn new(vol: VolumeHandle, bs: ondisk::Bootsector, mount_handle: ::vfs::mount::SelfHandle) -> ::vfs::Result<Box<InstanceWrapper>> {

		let cluster_size_bytes = bs.bytes_per_sector as usize * bs.sectors_per_cluster as usize;
		let cluster_size_blocks = cluster_size_bytes / vol.block_size();
		if cluster_size_bytes % vol.block_size() != 0 {
			log_error!("Unable to mount: cluster size ({:#x}) isn't a multiple of volume block size ({:#x})", cluster_size_bytes, vol.block_size());
		}
		// Pre-calculate some useful values (cluster size, mft entry, ...)
		let mut instance = Instance {
			vol: ::block_cache::CachedVolume::new(vol),
			mount_handle,
			mft_data_attr: None,
			cluster_size_blocks,
			mft_record_size: bs.mft_record_size.get().to_bytes(cluster_size_bytes),
			bs,
			};
		// Locate the (optional) MFT entry for the MFT
		instance.mft_data_attr = ::kernel::futures::block_on(instance.get_attr(ondisk::MFT_ENTRY_SELF, ondisk::FileAttr::Data, ondisk::ATTRNAME_DATA, /*index*/0))?;

		// SAFE: ArefInner::new requires a stable pointer, and the immediate boxing does that
		Ok(unsafe { Box::new(InstanceWrapper(aref::ArefInner::new(instance))) })
	}
}
impl ::vfs::mount::Filesystem for InstanceWrapper
{
	fn root_inode(&self) -> ::vfs::node::InodeId {
		ondisk::MFT_ENTRY_ROOT.0 as _
	}
	fn get_node_by_inode(&self, inode_id: ::vfs::node::InodeId) -> Option<::vfs::node::Node> {
		todo!("get_node_by_inode({})", inode_id)
	}
}

impl Instance
{
	pub async fn get_mft_entry(&self, entry: MftEntryIdx) -> ::vfs::Result<Box<ondisk::MftEntry>> {
		// TODO: Look up in a cache, and return `Arc<RwLock`
		self.load_mft_entry(entry).await
	}
	/// Look up a MFT entry
	pub async fn load_mft_entry(&self, entry_idx: MftEntryIdx) -> ::vfs::Result<Box<ondisk::MftEntry>> {
		let entry_idx = entry_idx.0;
		let mut buf = vec![0; self.mft_record_size];
		if let Some(ref e) = self.mft_data_attr {
			// Read from the attribute
			self.attr_read(e, 0, &mut buf).await?;
		}
		else {
			if self.mft_record_size > self.vol.block_size() {
				let blocks_per_entry = self.mft_record_size / self.vol.block_size();
				let blk = self.bs.mft_start * self.cluster_size_blocks as u64 + (entry_idx as usize * blocks_per_entry) as u64;
				self.vol.read_blocks(blk, &mut buf).await?;
			}
			else {
				let entries_per_block = (self.vol.block_size() / self.mft_record_size) as u32;
				let blk = self.bs.mft_start * self.cluster_size_blocks as u64 + (entry_idx / entries_per_block) as u64;
				let ofs = entry_idx % entries_per_block;
				self.vol.read_inner(blk, ofs as usize * self.mft_record_size, &mut buf).await?;
			}
		}
		Ok(ondisk::MftEntry::new_owned(buf))
	}
	///
	pub async fn get_attr(&self, entry: MftEntryIdx, attr_id: ondisk::FileAttr, name: &str, index: usize) -> ::vfs::Result<Option<ondisk::AttrHandle>> {
		// Get the MFT entry
		let mft_ent = self.get_mft_entry(entry).await?;
		// Iterate attributes
		let mut count = 0;
		for attr in mft_ent.iter_attributes() {
			log_debug!("ty={:#x} name={:?}", attr.ty(), attr.name());
			if attr.ty() == attr_id as u32 && attr.name() == name {
				if count == index {
					return Ok(Some(mft_ent.attr_handle(attr, entry)));
				}
				count += 1;
			}
		}
		Ok(None)
	}

	pub async fn attr_read(&self, attr: &ondisk::AttrHandle, ofs: u64, dst: &mut [u8]) -> ::vfs::Result<usize> {
		todo!("Instance::attr_read");
	}
}
