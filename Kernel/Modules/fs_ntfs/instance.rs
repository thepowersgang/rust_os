/**!
 * Core parts of a mounted NTFS volume
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

/// An instance (common information) for a mounted volume
pub struct Instance
{
	vol: ::block_cache::CachedVolume,
	_mount_handle: ::vfs::mount::SelfHandle,
	cluster_size_blocks: usize,
	mft_record_size: usize,
	mft_data_attr: Option<(CachedMft,ondisk::AttrHandle)>,
	upcase_table: Vec<u16>,
	bs: ondisk::Bootsector,

	// A cache of loaded (open and just in-cache) MFT entries
	mft_cache: ::kernel::sync::RwLock<::kernel::lib::VecMap<u32, CachedMft>>,
}

impl Instance
{
	/// Construct a new instance using a bootsector (`bs`) read from a volume (`vol`)
	pub fn new(vol: VolumeHandle, bs: ondisk::Bootsector, mount_handle: ::vfs::mount::SelfHandle) -> ::vfs::Result<Box<InstanceWrapper>> {

		let cluster_size_bytes = bs.bytes_per_sector as usize * bs.sectors_per_cluster as usize;
		let cluster_size_blocks = cluster_size_bytes / vol.block_size();
		if cluster_size_bytes % vol.block_size() != 0 {
			log_error!("Unable to mount: cluster size ({:#x}) isn't a multiple of volume block size ({:#x})", cluster_size_bytes, vol.block_size());
			return Err(::vfs::Error::InconsistentFilesystem);
		}
		// Pre-calculate some useful values (cluster size, mft entry, ...)
		let mut instance = Instance {
			vol: ::block_cache::CachedVolume::new(vol),
			_mount_handle: mount_handle,
			mft_data_attr: None,
			cluster_size_blocks,
			mft_record_size: bs.mft_record_size.get().to_bytes(cluster_size_bytes),
			bs,
			upcase_table: Vec::new(),
			mft_cache: Default::default(),
			};
		log_debug!("cluster_size_blocks = {:#x}, mft_record_size = {:#x}", instance.cluster_size_blocks, instance.mft_record_size);
		// Check that the driver can do a kinda-evil trick to cache MFT entries efficiently
		if let None = new_mft_cache_ent(instance.mft_record_size) {
			log_error!("Unable to mount: MFT record size too large for internal hackery ({:#x})", instance.mft_record_size);
			return Err(::vfs::Error::InconsistentFilesystem);
		}
		// Locate the (optional) MFT entry for the MFT
		instance.mft_data_attr = ::kernel::futures::block_on(instance.get_attr(ondisk::MFT_ENTRY_SELF, ondisk::FileAttr::Data, ondisk::ATTRNAME_DATA, /*index*/0))?;

		if let Some( (upcase_ent,upcase_data) ) = ::kernel::futures::block_on(instance.get_attr(ondisk::MFT_ENTRY_UPCASE, ondisk::FileAttr::Data, ondisk::ATTRNAME_DATA, 0))?
		{
			instance.upcase_table = {
				let mut upcase_table = vec![0u16; 0x10000];
				let len = ::kernel::futures::block_on(instance.attr_read(&upcase_ent, &upcase_data, 0, ::kernel::lib::as_byte_slice_mut(&mut upcase_table[..])))?;
				if len != 0x1_0000 * 2 {
					log_warning!("$UpCase not large enough - Read {:#x} bytes, expected {:#x}",
						len, upcase_table.len() * 2);
				}
				for e in upcase_table.iter_mut() {
					*e = e.to_le();
				}
				upcase_table
				};
		}

		// SAFE: ArefInner::new requires a stable pointer, and the immediate boxing does that
		Ok(unsafe { Box::new(InstanceWrapper(aref::ArefInner::new(instance))) })
	}

	fn cluster_size_bytes(&self) -> usize {
		self.cluster_size_blocks * self.vol.block_size()
	}

	/// Apply `update_sequence` fixups to a loaded metadata block
	///
	/// All metadata blocks (e.g. MFT entries, or index blocks) have an "update sequence" that catches sectors (volume
	/// blocks) that didn't get written to disk correctly. Within the first sector there's a sequence number that's
	/// incremented on every change to the block and a copy of the original/correct last two bytes of each sector.
	///
	/// This function takes that update sequence information, checks that the last word of each non-first sector matches
	/// the expectation and then restores the original value.
	///
	/// `get_usa` is a function that gets an `UpdateSequence` from the passed first sector
	pub fn apply_sequence_fixups(&self, buf: &mut [u8], get_usa: &dyn Fn(&[u8])->Option<&crate::ondisk::UpdateSequence>) -> Result<(),::vfs::Error> {
		let block_size = self.vol.block_size();
		if buf.len() >= block_size
		{
			assert!(buf.len() % block_size == 0, "apply_sequence_fixups: Passed a buffer not a multiple of volume blocks long"); 
			let (buf1, buf2) = buf.split_at_mut(block_size);
			let usa = (get_usa)(buf1).ok_or_else(|| { log_error!("get_usa failed"); ::vfs::Error::InconsistentFilesystem })?;
			let exp_val = usa.sequence_number();
			let mut usa_it = usa.array();
			let s0_last_word = usa_it.next().ok_or(::vfs::Error::InconsistentFilesystem)?;
			//log_debug!("apply_sequence_fixups: {} sectors, seq={}", 1+buf2.len()/block_size, exp_val);

			fn apply_fixup(sector_idx: usize, sector: &mut [u8], last_word: u16, exp_val: u16) -> Result<(),::vfs::Error> {
				let block_size = sector.len();
				let slot = &mut sector[ block_size - 2 ..];
				let cur_val = u16::from_le_bytes([slot[0], slot[1]]);
				//log_debug!("apply_sequence_fixups: +{}: 0x{:04x} -> 0x{:04x}", 1+sector_idx, cur_val, last_word);
				if cur_val != exp_val {
					log_error!("apply_sequence_fixups: Sequence number mismatch in sector +{}: 0x{:04x} != exp 0x{:04x}",
						1+sector_idx, cur_val, exp_val
						);
					return Err(::vfs::Error::InconsistentFilesystem);
				}
				slot.copy_from_slice(&last_word.to_le_bytes());
				Ok( () )
			}

			for (rel_sector_idx, (sector, last_word)) in Iterator::zip( buf2.chunks_mut(block_size), usa_it ).enumerate()
			{
				apply_fixup(1+rel_sector_idx, sector, last_word, exp_val)?;
			}

			apply_fixup(0, &mut buf[..block_size], s0_last_word, exp_val)?;
		}
		else
		{
			// Call the getter, just so the error is triggered if needed
			match (get_usa)(buf) {
			Some(_) => {},
			None => {
				log_error!("apply_sequence_fixups: `get_usa` failed");
				return Err(::vfs::Error::InconsistentFilesystem);
				}
			};
		}
		Ok( () )
	}

	pub fn compare_ucs2_nocase(&self, a: u16, b: u16) -> ::core::cmp::Ordering {
		// Look up $UpCase
		if self.upcase_table.len() == 0x1_0000 {
			let a = self.upcase_table[a as usize];
			let b = self.upcase_table[b as usize];
			::core::cmp::Ord::cmp(&a, &b)
		}
		else {
			fn to_upper(v: u16) -> u16 {
				match v {
				0x61 ..= 0x7A => v - 0x20,
				_ => v,
				}
			}
			let a = to_upper(a);
			let b = to_upper(b);
			::core::cmp::Ord::cmp(&a, &b)
		}
	}
	pub fn compare_ucs2_nocase_iter(&self, a: &mut dyn Iterator<Item=u16>, b: &mut dyn Iterator<Item=u16>) -> ::core::cmp::Ordering {
		// TODO: Case insensitive UCS-2 comparison! - use the `$UpCase` special file to obtain the sorting rule to use
		use ::core::cmp::Ordering;
		loop {
			match (a.next(), b.next())
			{
			(None,None) => return Ordering::Equal,
			(None,Some(_)) => return Ordering::Less,
			(Some(_),None) => return Ordering::Greater,
			(Some(a),Some(b)) => match self.compare_ucs2_nocase(a,b)
				{
				Ordering::Equal => {},
				v => return v,
				},
			}
		}
	}
}
impl ::vfs::mount::Filesystem for InstanceWrapper
{
	fn root_inode(&self) -> ::vfs::node::InodeId {
		ondisk::MFT_ENTRY_ROOT.0 as _
	}
	fn get_node_by_inode(&self, inode_id: ::vfs::node::InodeId) -> Option<::vfs::node::Node> {
		let ent = match ::kernel::futures::block_on(self.0.get_mft_entry(MftEntryIdx(inode_id as _)))
			{
			Err(_) => return None,
			Ok(v) => v,
			};
		// Check the node type
		if ent.inner.read().flags_isdir()
		{
			Some(::vfs::node::Node::Dir(Box::new(super::dir::Dir::new(self.0.borrow(), inode_id, ent))))
		}
		else
		{
			// How are symlinks or directory junctions handled?
			Some(::vfs::node::Node::File(Box::new(super::file::File::new(self.0.borrow(), inode_id, ent))))
		}
	}
}

/**
 * MFT entries and attributes - the core of a NTFS driver
 */
impl Instance
{
	/// Obtain a handle to an NTFS entry in the cache
	pub async fn get_mft_entry(&self, entry: MftEntryIdx) -> ::vfs::Result<CachedMft> {
		// Look up in a cache, and return `Arc<RwLock`
		{
			let lh = self.mft_cache.read();
			if let Some(v) = lh.get(&entry.0) {
				return Ok(v.clone());
			}
		}
		let rv = self.load_mft_entry_dyn(entry).await?;
		
		{
			log_debug!("MFT Entry #{}", entry.0);
			let m = rv.inner.read();
			for attr in m.iter_attributes() {
				log_debug!("Attribute: ty={} name={:?}", crate::ondisk::FileAttr::fmt_val(attr.ty()), attr.name());
				//log_debug!("{}", attr.name() as *const _ as *const u8 as usize - &*m as *const _ as *const () as usize);
			}
		}
		{
			let mut lh = self.mft_cache.write();
			let rv = lh.entry(entry.0).or_insert(rv).clone();
			// TODO: Prune the cache?
			//lh.retain(|k,v| Arc::strong_count(v) > 1);
			Ok(rv)
		}
	}

	/// Load a MFT entry from the disk (this is a wrapper that avoids recursion with `attr_read` by boxing an erasing the future)
	fn load_mft_entry_dyn(&self, entry_idx: MftEntryIdx) -> ::core::pin::Pin<Box< dyn ::core::future::Future<Output=::vfs::Result<CachedMft>> + '_ >> {
		Box::pin(self.load_mft_entry(entry_idx))
	}
	/// Load a MFT entry from the disk
	async fn load_mft_entry(&self, entry_idx: MftEntryIdx) -> ::vfs::Result<CachedMft> {
		let entry_idx = entry_idx.0;
		// TODO: Check that `entry_idx` is within the valid range for the MFT

		let mut rv_bytes = new_mft_cache_ent(self.mft_record_size).expect("Unexpected record size");
		let buf = ::kernel::lib::mem::Arc::get_mut(&mut rv_bytes).unwrap().inner.get_mut();
		if let Some((ref mft_ent, ref e)) = self.mft_data_attr {
			// Read from the attribute
			let l = self.attr_read(mft_ent, e, entry_idx as u64 * self.mft_record_size as u64, buf).await?;
			if l == 0 {
				// Zero read length means that the read was past the end?
				return Err(::vfs::Error::NotFound);
			}
			assert!(l == buf.len(), "Partial read of MFT entry? ({} != {})", l, buf.len());
		}
		else {
			if self.mft_record_size > self.vol.block_size() {
				let blocks_per_entry = self.mft_record_size / self.vol.block_size();
				let blk = self.bs.mft_start * self.cluster_size_blocks as u64 + (entry_idx as usize * blocks_per_entry) as u64;
				self.vol.read_blocks(blk, buf).await?;
			}
			else {
				let entries_per_block = (self.vol.block_size() / self.mft_record_size) as u32;
				let blk = self.bs.mft_start * self.cluster_size_blocks as u64 + (entry_idx / entries_per_block) as u64;
				let ofs = entry_idx % entries_per_block;
				self.vol.read_inner(blk, ofs as usize * self.mft_record_size, buf).await?;
			}
		}

		// Apply sequence number fixups
		//log_debug!("{:?}", ::kernel::logging::HexDump(&*buf));
		self.apply_sequence_fixups(buf, &|buf1| {
			ondisk::MftEntry::new_borrowed(buf1).map(|ent| ent.update_sequence())
			})?;
		//log_debug!("{:?}", ::kernel::logging::HexDump(&*buf));

		// SAFE: `MftEntry` and `[u8]` have the same representation
		Ok(unsafe { ::core::mem::transmute(rv_bytes) })
	}
	/// Get a handle to an attribute within a MFT entry
	// TODO: How to handle invalidation of the attribute info when the MFT entry is updated (at least, when an attribute is resized)
	// - Could have attribute handles be indexes into a pre-populated list
	pub async fn get_attr(&self, entry: MftEntryIdx, attr_id: ondisk::FileAttr, name: &str, index: usize) -> ::vfs::Result<Option<(CachedMft, ondisk::AttrHandle)>> {
		// Get the MFT entry
		let e = self.get_mft_entry(entry).await?;
		let rv = self.get_attr_inner(&e, attr_id, name, index);
		Ok(rv.map(|a| (e, a)))
	}

	/// Get a handle to an attribute within a MFT entry
	pub fn get_attr_inner(&self, mft_ent: &CachedMft, attr_id: ondisk::FileAttr, name: &str, index: usize) -> Option<ondisk::AttrHandle> {
		let mft_ent = mft_ent.inner.read();
		// Iterate attributes
		let mut count = 0;
		for attr in mft_ent.iter_attributes() {
			if attr.ty() == attr_id as u32 && attr.name() == name {
				if count == index {
					return Some(mft_ent.attr_handle(attr));
				}
				count += 1;
			}
		}
		None
	}

	/// Query the current size of the attribute's data
	pub fn attr_size(&self, mft_ent: &CachedMft, attr: &ondisk::AttrHandle) -> u64 {
		let mft_ent = mft_ent.inner.read();
		let Ok(a) = mft_ent.get_attr(attr).ok_or(::vfs::Error::Unknown("Stale ntfs AttrHandle")) else { return 0; };
		match a.inner()
		{
		ondisk::MftAttribData::Resident(r) => r.data().len() as u64,
		ondisk::MftAttribData::Nonresident(r) => r.real_size(),
		}
	}

	/// Read data out of an attribute (resident or non-resident)
	pub async fn attr_read(&self, mft_ent: &CachedMft, attr: &ondisk::AttrHandle, ofs: u64, mut dst: &mut [u8]) -> ::vfs::Result<usize> {
		if dst.len() == 0 {
			return Ok(0);
		}

		let mft_ent = mft_ent.inner.read();
		let a = mft_ent.get_attr(attr).ok_or(::vfs::Error::Unknown("Stale ntfs AttrHandle"))?;
		match a.inner()
		{
		ondisk::MftAttribData::Resident(r) => {
			let src = r.data();
			if ofs > src.len() as u64 {
				return Err(::vfs::Error::InvalidParameter);
			}
			let src = &src[ofs as usize..];
			let len = usize::min( src.len(), dst.len() );
			dst[..len].copy_from_slice(&src[..len]);
			Ok(len)
			},
		ondisk::MftAttribData::Nonresident(r) => {
			if false {
				log_debug!("RelSize: {:#x} (ofs={:#x})", r.real_size(), ofs);
				log_debug!("VCNs: {} -- {}", r.starting_vcn(), r.last_vcn());
				for run in r.data_runs() {
					log_debug!("Data Run: {:x?} + {}", run.lcn, run.cluster_count);
				}
			}

			// Check the valid size
			if ofs > r.real_size() {
				return Err(::vfs::Error::InvalidParameter)
			}
			// Clamp the data size
			let space = r.real_size() - ofs;
			if space < dst.len() as u64 {
				dst = &mut dst[..space as usize];
			}

			if r.starting_vcn() != 0 {
				todo!("attr_read: TODO - Handle sparse files (starting_vcn = {})", r.starting_vcn());
				// For this, inject a run filled with zeroes?
			}

			let mut cur_vcn = ofs / (self.cluster_size_bytes() as u64);
			let mut cur_ofs = ofs as usize % self.cluster_size_bytes();

			let mut runs = CompressionRuns::new(r.data_runs(), 1 << r.compression_unit_size()).peekable();
			// Seek to the run containing the first cluster
			let mut runbase_vcn = 0;
			while let Some(r) = runs.peek() {
				if runbase_vcn + r.cluster_count() > cur_vcn {
					break;
				}
				runbase_vcn += r.cluster_count();
				runs.next();
			}
			let rv = dst.len();
			// Keep consuming runs until the destination is empty
			while dst.len() > 0
			{
				assert!(cur_vcn >= runbase_vcn, "cur_vcn({}) >= runbase_vcn({})", cur_vcn, runbase_vcn);
				let Some(cur_run) = runs.next() else {
					// Filled with zeroes? Or invalid parameter?
					todo!("Handle reading past the end of the populated runs");
					};

				match cur_run
				{
				CompressionRun::Sparse(run_cluster_count) => {
					log_debug!("Sparse +{}", run_cluster_count);
					// VCN within the run
					let rel_vcn = cur_vcn - runbase_vcn;
					// Number of clusters available in the run
					let cluster_count = run_cluster_count - rel_vcn;
					// Number of bytes we can read in this loop
					let len = usize::min(dst.len(), (cluster_count as usize) * self.cluster_size_bytes() - cur_ofs);
					let buf = ::kernel::lib::split_off_front_mut(&mut dst, len).unwrap();

					buf.fill(0);

					runbase_vcn += run_cluster_count;
					cur_vcn += cluster_count;
					cur_ofs = 0;
					},
				CompressionRun::Raw(crun_cluster_count, iter) => {
					log_debug!("Raw +{}", crun_cluster_count);
					let mut iter = iter.peekable();
					let mut irunbase_vcn = runbase_vcn;
					while let Some(r) = iter.peek() {
						if irunbase_vcn + r.cluster_count > cur_vcn {
							break;
						}
						irunbase_vcn += r.cluster_count;
						iter.next();
					}
					while let Some(cur_run) = iter.next()
					{
						let run_lcn = cur_run.lcn.expect("CompressionRun::Raw with sparse run");

						if dst.len() == 0 {
							break ;
						}

						// VCN within the run
						assert!(cur_vcn >= irunbase_vcn, "cur_vcn({}) >= irunbase_vcn({})", cur_vcn, irunbase_vcn);
						let rel_vcn = cur_vcn - irunbase_vcn;
						// Number of clusters available in the run
						let cluster_count = cur_run.cluster_count - rel_vcn;
						// Number of bytes we can read in this loop
						let len = usize::min(dst.len(), (cluster_count as usize) * self.cluster_size_bytes() - cur_ofs);
						let buf = ::kernel::lib::split_off_front_mut(&mut dst, len).unwrap();

						let lcn = run_lcn + rel_vcn;
						let block = lcn * self.cluster_size_blocks as u64 + (cur_ofs / self.vol.block_size()) as u64;
						let block_ofs = cur_ofs % self.vol.block_size();
						if block_ofs != 0 || buf.len() % self.vol.block_size() != 0 {
							self.vol.read_inner(block, block_ofs, buf).await?;
						}
						else {
							self.vol.read_blocks(block, buf).await?;
						}

						irunbase_vcn += cur_run.cluster_count;
						cur_vcn += cluster_count;
						assert!(cur_vcn == irunbase_vcn);
						cur_ofs = 0;
					}
					let n_zeroes = crun_cluster_count - (irunbase_vcn - runbase_vcn);
					if n_zeroes > 0 {
						for _ in 0 .. n_zeroes {
							let len = self.cluster_size_bytes();
							let buf = ::kernel::lib::split_off_front_mut(&mut dst, len).unwrap();
							buf.fill(0);
							cur_vcn += 1;
							irunbase_vcn += 1;
						}
					}
					runbase_vcn += crun_cluster_count;
					assert!(cur_vcn == runbase_vcn, "{cur_vcn} != {runbase_vcn}, len={}", dst.len());
					},
				CompressionRun::Compressed(uncompressed_count, compressed_count, iter) => {
					log_debug!("Compressed +{}", compressed_count);
					// Iterate compressed blocks, and decompress into the target buffer (or a bounce buffer - if incomplete)
					// - Load the entire compression unit? (Or, stream in pairs of 8K chunks)
					let mut buf = vec![ 0u8; compressed_count as usize * self.cluster_size_bytes() ];
					{
						let mut dst = &mut buf[..];
						for cur_run in iter
						{
							let len = usize::min( dst.len(), cur_run.cluster_count as usize * self.cluster_size_bytes() );
							let buf = ::kernel::lib::split_off_front_mut(&mut dst, len).unwrap();
							let lcn = cur_run.lcn.expect("CompressionRun::Compressed with a sparse run");
							let block = lcn * self.cluster_size_blocks as u64;
							self.vol.read_blocks(block, buf).await?;
						}
					}
					if false
					{
						let mut decomp = crate::compression::Decompressor::new(&buf);
						while let Some(len) = decomp.get_block(None) {
							log_debug!("get_block length: {}", len);
						}
					}

					// - Iterate through compressed blocks (4K) skipping until target data
					let mut decomp = crate::compression::Decompressor::new(&buf);
					let rel_vcn = cur_vcn - runbase_vcn;
					let byte_ofs = rel_vcn as usize * self.cluster_size_bytes() + cur_ofs;

					// Ensure that `dst` here just contains the buffer to be used for this compression unit
					// - That avoids accidentally over-reading (which could happen if there are extra compressed bytes)
					let mut dst = {
						let maxlen = uncompressed_count as usize * self.cluster_size_bytes() - byte_ofs;
						let len = usize::min(dst.len(), maxlen);
						::kernel::lib::split_off_front_mut(&mut dst, len).unwrap()
						};

					const BLOCK_SIZE: usize = 0x1000;
					// Consume complete blocks until the offset is reached
					for _ in 0 .. byte_ofs / BLOCK_SIZE
					{
						match decomp.get_block(None)
						{
						Some(BLOCK_SIZE) => {},
						v => {
							log_error!("Inconsistent filesystem: Encountered end of compressed data while seeking to byte_ofs={}: {:?}", byte_ofs, v);
							return Err(::vfs::Error::InconsistentFilesystem)?
							},
						}
					}

					// If the read position ends up within a compressed block - read into a temporary buffer
					// and then read from part of that buffer
					if byte_ofs % BLOCK_SIZE != 0
					{
						let byte_ofs = byte_ofs % BLOCK_SIZE;
						let mut uc_block = vec![0u8; BLOCK_SIZE];
						let Some(len) = decomp.get_block(Some(&mut uc_block)) else {
							log_error!("Inconsistent filesystem: Encountered end of compressed data while seeking to byte_ofs={}: partial block", byte_ofs);
							return Err(::vfs::Error::InconsistentFilesystem);
							};
						assert!(len <= BLOCK_SIZE, "{} > {}", len, BLOCK_SIZE);
						// This can be a partial block, if it's the final block in the file
						if len < byte_ofs {
							// Inconsistent: The offset is already clamped to within the valid initialised range of the file, so this shouldn't happen
							log_error!("Inconsistent filesystem: Partial compressed chunk of {} bytes, byte_ofs={}", len, byte_ofs);
							return Err(::vfs::Error::InconsistentFilesystem);
						}

						let len = usize::min(len - byte_ofs, dst.len());
						let b = ::kernel::lib::split_off_front_mut(&mut dst, len).unwrap();
						b.copy_from_slice(&uc_block[byte_ofs..][..len]);
					}

					// Consume entire (or leading partial) compression chunks
					while dst.len() > 0
					{
						let Some(len) = decomp.get_block(Some(dst)) else {
							log_error!("Inconsistent filesystem: Unexpected end of compressed data");
							return Err(::vfs::Error::InconsistentFilesystem);
							};
						let len = usize::min(len, dst.len());
						::kernel::lib::split_off_front_mut(&mut dst, len).unwrap();
					}
					},
				}
			}

			Ok(rv)
			},
		}
	}
}

pub type CachedMft = ::kernel::lib::mem::Arc< MftCacheEnt<ondisk::MftEntry> >;

pub struct MftCacheEnt<T: ?Sized> {
	//count: ::core::sync::atomic::AtomicUsize,
	inner: ::kernel::sync::RwLock<T>,
}
impl<T> MftCacheEnt<T> {
	pub fn new(inner: T) -> Self {
		Self {
			//count: Default::default(),
			inner: ::kernel::sync::RwLock::new(inner),
		}
	}
}
impl<T: ?Sized + Send + Sync> MftCacheEnt<T>
{
	pub fn read(&self) -> ::kernel::sync::rwlock::Read<'_, T> {
		self.inner.read()
	}
}
/// An evil hack to get a `Arc<Wrapper<MftEntry>>`
fn new_mft_cache_ent(mft_size: usize) -> Option< ::kernel::lib::mem::Arc<MftCacheEnt<[u8]>> > {
	use ::kernel::lib::mem::Arc;
	fn new_ce<const N: usize>() -> Arc<MftCacheEnt<[u8]>> {
		Arc::new(MftCacheEnt::new([0u8; N]))
	}
	let rv = match mft_size.next_power_of_two().ilog2()
		{
		0 ..= 4 |
		5  => new_ce::<{1<< 5}>(),	// 32
		6  => new_ce::<{1<< 6}>(),
		7  => new_ce::<{1<< 7}>(),
		8  => new_ce::<{1<< 8}>(),
		9  => new_ce::<{1<< 9}>(),	// 512
		10 => new_ce::<{1<<10}>(),
		11 => new_ce::<{1<<11}>(),	// 2048
		_ => return None,
		};
	Some(rv)
}

struct CompressionRuns<'a> {
	num_clusters_per_block: u64,
	// Most recent item popped from `end`
	cur: Option<(u64, crate::ondisk::DataRun)>,
	// Iterator to `cur`
	start: crate::ondisk::DataRunsIt<'a>,
	// Iterator after `cur`
	end: crate::ondisk::DataRunsIt<'a>,
}
impl<'a> CompressionRuns<'a> {
	pub fn new(mut inner: crate::ondisk::DataRunsIt<'a>, num_clusters_per_block: u64) -> Self {
		Self {
			num_clusters_per_block,
			start: inner.clone(),
			cur: inner.next().map(|v| (0, v)),
			end: inner,
		}
	}
}
impl<'a> Iterator for CompressionRuns<'a> {
	type Item = CompressionRun<'a>;
	fn next(&mut self) -> Option<CompressionRun<'a>> {
		let Some( (ref mut ofs, ref dr) ) = self.cur else { return None; };
		// If `num_clusters_per_block` is 1, then the data is uncompressed (but might still be sparse)
		if self.num_clusters_per_block == 1 {
			let dri = CompressionDataRuns {
				stream: self.start.clone(),
				ofs: Some(0),
				run_count: 0,
				};
			let dr = *dr;
			self.start = self.end.clone();
			self.cur = self.end.next().map(|v| (0, v));
			return Some(match dr.lcn {
				None => CompressionRun::Sparse(dr.cluster_count),
				Some(_lcn) => CompressionRun::Raw(dr.cluster_count, dri),
			})
		}
		// Non-unit compression unit size, so need to handle compression
		// Look for spans of `Raw(n) Sparse(num_clusters_per_block-n)` (accounting for partial spans)
		let blocks_avail = dr.cluster_count - *ofs;
		Some(match dr.lcn
		{
		None => {
			// A sparse block in `cur` just means a sparse run - doesn't indicate compression
			self.start = self.end.clone();
			self.cur = self.end.next().map(|v| (0, v));
			CompressionRun::Sparse(blocks_avail)
			},
		Some(_lcn) if blocks_avail >= self.num_clusters_per_block => {
			// There is a whole block left in this run
			let block_count = blocks_avail / self.num_clusters_per_block * self.num_clusters_per_block;
			let dri = CompressionDataRuns {
				stream: self.start.clone(),
				ofs: Some({ let v = *ofs; *ofs += block_count; v }),
				run_count: 0,
				};
			if *ofs == dr.cluster_count {
				self.start = self.end.clone();
				self.cur = self.end.next().map(|v| (0, v));
			}
			CompressionRun::Raw(block_count, dri)
			}
		Some(_lcn) => {
			let mut dri = CompressionDataRuns {
				stream: self.start.clone(),
				ofs: Some(*ofs),
				run_count: 0
				};
			let mut blocks_avail = blocks_avail;
			loop {
				let new_start = self.end.clone();
				// Partial block - get the next one from the `end`
				match self.end.next()
				{
				None => {
					// Partial block at the end - raw data
					self.cur = None;
					break CompressionRun::Raw(blocks_avail, dri);
					},
				Some(new_run) if new_run.lcn.is_none() => {
					// Populated followed by a sparse, must be compressed
					// - Note: Clamp is a defensive check, as this doesn't actually check for this sparse being enough to round out the block
					let new_ofs = (self.num_clusters_per_block - blocks_avail).clamp(0, new_run.cluster_count);
					if new_ofs != self.num_clusters_per_block - blocks_avail {
						// Warning? This means inconsistent filesystem state (the sparse run should be long enough)
						// - Is `Raw(n) Sparse(1) Sparse(m)` where `n+1+m=num_clusters_per_block` valid?
					}
					self.cur = Some((new_ofs, new_run));
					self.start = new_start;
					break CompressionRun::Compressed(self.num_clusters_per_block, blocks_avail, dri);
					},
				Some(new_run) if new_run.cluster_count + blocks_avail >= self.num_clusters_per_block => {
					// Disjoint uncompressed run
					self.cur = Some((self.num_clusters_per_block - blocks_avail, new_run));
					self.start = new_start;
					break CompressionRun::Raw(self.num_clusters_per_block, dri);
					},
				Some(new_run) => {
					// Not yet enough data to know if compression is present, keep track of how many populated clusters are present
					blocks_avail += new_run.cluster_count;
					dri.run_count += 1;
					},
				}
			}
			},
		})
	}
}
enum CompressionRun<'a> {
	Sparse(u64),
	Raw(u64, CompressionDataRuns<'a>),
	// Note: The cluster count here is the UNCOMPRESSED count
	Compressed(u64, u64, CompressionDataRuns<'a>),
}
impl<'a> CompressionRun<'a> {
	fn cluster_count(&self) -> u64 {
		match *self {
		CompressionRun::Sparse(c) => c,
		CompressionRun::Raw(c, _) => c,
		CompressionRun::Compressed(c, _, _) => c,
		}
	}
}
struct CompressionDataRuns<'a> {
	stream: crate::ondisk::DataRunsIt<'a>,
	ofs: Option<u64>,
	run_count: usize,
}
impl<'a> Iterator for CompressionDataRuns<'a> {
	type Item = crate::ondisk::DataRun;
	fn next(&mut self) -> Option<Self::Item> {
		if let Some(ofs) = self.ofs.take() {
			let mut n = self.stream.next().unwrap();
			n.cluster_count -= ofs;
			*n.lcn.as_mut().unwrap() -= ofs;
			Some(n)
		}
		else if self.run_count == 0 {
			None
		}
		else {
			self.run_count -= 1;
			self.stream.next()
		}
	}
}

