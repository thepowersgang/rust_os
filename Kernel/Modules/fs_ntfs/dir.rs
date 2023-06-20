
use ::kernel::lib::byte_str::ByteStr;

pub struct Dir {
	instance: super::instance::InstanceRef,
	mft_idx: u64,
	mft_ent: super::instance::CachedMft,
	
	i30_root: Option<super::ondisk::AttrHandle>,
	i30_allocation: Option<super::ondisk::AttrHandle>,
}
impl Dir
{
	pub fn new(instance: super::instance::InstanceRef, mft_idx: u64, mft_ent: super::instance::CachedMft) -> Self {
		Dir {
			i30_root: instance.get_attr_inner(&mft_ent, crate::ondisk::FileAttr::IndexRoot, "$I30", 0),
			i30_allocation: instance.get_attr_inner(&mft_ent, crate::ondisk::FileAttr::IndexAllocation, "$I30", 0),
			instance,
			mft_idx,
			mft_ent,
		}
	}

	fn get_root<'a>(&self, mft_ent: &'a crate::ondisk::MftEntry) -> Option<&'a crate::ondisk::Attrib_IndexRoot> {
		let Some(ref h) = self.i30_root else {
			log_warning!("$I30 IndexRoot missing");
			return None;
			};
		let Some(attr) = mft_ent.get_attr(h) else {
			log_error!("Unable to re-get attribute for $I30 IndexRoot!");
			return None;
			};
		let Some(resident) = attr.inner().as_resident() else {
			log_error!("TODO: $I30 IndexRoot not resident?");
			return None;
			};
		let data = resident.data();
		let Some(rv) = crate::ondisk::Attrib_IndexRoot::from_slice(data) else {
			log_error!("$I30 IndexRoot too small? len={}", data.len());
			return None;
			};
		Some(rv)
	}
}
impl ::vfs::node::NodeBase for Dir
{
	fn get_id(&self) -> u64 {
		self.mft_idx
	}
	fn get_any(&self) -> &(dyn ::core::any::Any + 'static) {
		self
	}
}
impl ::vfs::node::Dir for Dir
{
	fn lookup(&self, name: &ByteStr) -> Result<u64, ::vfs::Error> {
		let mft_ent = self.mft_ent.read();
		let i30_root = self.get_root(&mft_ent).ok_or(::vfs::Error::InconsistentFilesystem)?;

		// Returns either a found entry, or an error with an optional recursion VCN
		fn btree_search<'a>(hdr: &'a crate::ondisk::Attrib_IndexHeader, cmp: impl Fn(&[u8])->::core::cmp::Ordering) -> Result<&'a crate::ondisk::Attrib_IndexEntry, Option<u64>> {
			use ::core::cmp::Ordering;
			let mut data = hdr.entries_slice();
			while data.len() > 0
			{
				let Some(ent) = crate::ondisk::Attrib_IndexEntry::from_slice(data) else {
					return Err(None);
					};
				let subnode = ent.subnode_vcn();
				// Returns `none` if this is the last entry - the last entry has no data
				let Some(next) = ent.next() else {
					return Err(subnode);
					};
				match cmp(ent.data())
				{
				Ordering::Equal => return Ok(ent),
				Ordering::Less => {},
				Ordering::Greater => return Err(subnode),
				}

				data = next;
			}
			Err(None)
		}
		
		let cmp = |attr_data: &[u8]| {
			let Some(a) = crate::ondisk::Attrib_Filename::from_slice(attr_data) else { return ::core::cmp::Ordering::Less };
			self.instance.compare_ucs2_nocase_iter(&mut a.filename().iter_units(), &mut ::utf16::wtf8_to_utf16(name.as_bytes()))
			};
		let mut vcn = match btree_search(i30_root.index_header(), cmp)
			{
			Ok(e) => return Ok(e.mft_reference_num()),
			Err(None) => return Err(::vfs::Error::NotFound),
			Err(Some(v)) => v,
			};
		if i30_root.index_header().flags() & 0x1 == 0 {
			// Explicit error? The root had a chain VCN, but no allocation was expected present
		}
		let i30_alloc = self.i30_allocation.as_ref().ok_or(::vfs::Error::InconsistentFilesystem)?;
		let mut buf = vec![ 0; i30_root.index_block_size() as usize];
		loop
		{
			let l = ::kernel::futures::block_on(self.instance.attr_read(&self.mft_ent, i30_alloc, vcn * buf.len() as u64, &mut buf))?;
			if l == 0 {
				// Inconsistent? Off the end
				return Err(::vfs::Error::NotFound);
			}
			let block_hdr = get_index_block(&self.instance, &mut buf[..l])?;
			vcn = match btree_search(block_hdr.index_header(), cmp)
				{
				Ok(e) => return Ok(e.mft_reference_num()),
				Err(None) => return Err(::vfs::Error::NotFound),
				Err(Some(v)) => v,
				};
		}
	}
	fn read(&self, ofs: usize, cb: &mut ::vfs::node::ReadDirCallback) -> Result<usize, ::vfs::Error> {
		let mft_ent = self.mft_ent.read();
		let i30_root = self.get_root(&mft_ent).ok_or(::vfs::Error::InconsistentFilesystem)?;
		// Iterate index entries
		// - Start with the information in the root (which should be resident)
		
		fn iterate_index<'a>(hdr: &'a crate::ondisk::Attrib_IndexHeader, pos: &mut usize) -> Option<&'a crate::ondisk::Attrib_IndexEntry> {
			let mut data = hdr.entries_slice();
			while data.len() > 0
			{
				let Some(ent) = crate::ondisk::Attrib_IndexEntry::from_slice(data) else {
					break
					};
				//log_debug!("iterate_index: MFTRef={:#x} flags={:#x} data={:02x?}", ent.mft_reference(), ent.index_flags(), ent.data());
				// Returns `none` if this is the last entry - the last entry has no data
				let Some(next) = ent.next() else {
					break;
					};
				if *pos == 0 {
					return Some(ent);
				}
				*pos -= 1;
				data = next;
			}
			None
		}

		// TODO: Have `ofs` be a byte offset (or something that doesn't require linear iteration on each run)
		let mut pos = ofs;
		let mut rv = ofs;
		{
			while let Some(v) = iterate_index(i30_root.index_header(), &mut pos) {
				let a = crate::ondisk::Attrib_Filename::from_slice(v.data()).ok_or(::vfs::Error::InconsistentFilesystem)?;
				log_debug!("Dir::read: Found (root) {:?}", a.filename());
				rv += 1;
				pos = rv;
				if ! cb(v.mft_reference_num(), &mut a.filename().wtf8()) {
					return Ok(rv);
				}
			}
		}

		// If this flag is set, the index doesn't fit in the root
		if i30_root.index_header().flags() & 0x1 != 0
		{
			let i30_alloc = self.i30_allocation.as_ref().ok_or(::vfs::Error::InconsistentFilesystem)?;

			let mut buf = vec![ 0; i30_root.index_block_size() as usize];
			let mut ipos = pos;
			for read_ofs in (0 ..).step_by(buf.len())
			{
				let l = ::kernel::futures::block_on(self.instance.attr_read(&self.mft_ent, i30_alloc, read_ofs, &mut buf))?;
				if l == 0 {
					break;
				}
				//log_debug!("Alloc block @{:#x}: {:?}", read_ofs, ::kernel::logging::HexDump(&buf[..128]));
				let block_hdr = get_index_block(&self.instance, &mut buf[..l])?;
				let index_hdr = block_hdr.index_header();

				while let Some(v) = iterate_index(index_hdr, &mut pos) {
					//log_debug!("Indexed attribute: {:?}", ::kernel::logging::HexDump(v.data()));
					let a = crate::ondisk::Attrib_Filename::from_slice(v.data()).ok_or(::vfs::Error::InconsistentFilesystem)?;
					log_debug!("Dir::read: Found {:?}", a.filename());
					rv += 1;
					ipos += 1;
					pos = ipos;
					if ! cb(v.mft_reference_num(), &mut a.filename().wtf8()) {
						return Ok(rv);
					}
				}
			}
		}

		Ok(rv)
	}
	fn create(&self, _name: &ByteStr, _node_type: ::vfs::node::NodeType<'_>) -> Result<u64, ::vfs::Error> {
		Err(::vfs::Error::ReadOnlyFilesystem)
	}
	fn link(&self, _name: &ByteStr, _node: &dyn ::vfs::node::NodeBase) -> Result<(), ::vfs::Error> {
		Err(::vfs::Error::ReadOnlyFilesystem)
	}
	fn unlink(&self, _name: &ByteStr) -> Result<(), ::vfs::Error> {
		Err(::vfs::Error::ReadOnlyFilesystem)
	}
}

fn get_index_block<'a>(instance: &super::instance::Instance, buf: &'a mut [u8]) -> Result<&'a crate::ondisk::Attrib_IndexBlockHeader, ::vfs::Error>
{
	instance.apply_sequence_fixups(buf, &|buf1| crate::ondisk::Attrib_IndexBlockHeader::from_slice(buf1).map(|ent| ent.update_sequence()))?;
	crate::ondisk::Attrib_IndexBlockHeader::from_slice(buf).ok_or(::vfs::Error::InconsistentFilesystem)
}


