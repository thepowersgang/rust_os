// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/fs_fat/dir.rs
use kernel::prelude::*;
use kernel::lib::mem::aref::ArefBorrow;
use kernel::lib::byte_str::ByteStr;
use ::vfs::{self, node};
use super::on_disk;
use super::file::FileNode;
use super::ClusterList;
use super::ClusterNum;
use super::FilesystemInner;
use utf16::Str16;

pub struct DirNode
{
	fs: ArefBorrow<crate::FilesystemInner>,
	start_cluster: ClusterNum,
}
impl_fmt! {
	Debug(self, f) for DirNode {
		write!(f, "{{cluster={:?}}}", self.start_cluster)
	}
}

impl DirNode {
	pub fn new(fs: ArefBorrow<FilesystemInner>, start_cluster: ClusterNum) -> DirNode {
		DirNode {
			fs: fs,
			start_cluster: start_cluster,
		}
	}
	pub fn new_boxed(fs: ArefBorrow<FilesystemInner>, start_cluster: ClusterNum) -> Box<DirNode> {
		Box::new(Self::new(fs, start_cluster))
	}
}

impl node::NodeBase for DirNode {
	fn get_id(&self) -> node::InodeId {
		todo!("DirNode::get_id")
	}
	fn get_any(&self) -> &dyn core::any::Any {
		self
	}
}

#[derive(Debug)]
pub struct OpenFileInfo
{
	dir_cluster: ClusterNum,
	reference_count: u32,
}
impl OpenFileInfo {
	pub fn new(dir_cluster: ClusterNum) -> OpenFileInfo {
		OpenFileInfo {
			dir_cluster,
			reference_count: 0,
		}
	}
	pub fn add_ref(&mut self) {
		self.reference_count += 1;
	}
	/// Returns `true` if the reference count is now zero
	pub fn sub_ref(&mut self) -> bool {
		self.reference_count -= 1;
		self.reference_count == 0
	}
}

impl super::FilesystemInner {
	fn get_dir_info(&self, cluster: ClusterNum) -> DirInfoHandle {
		DirInfoHandle {
			fs: self,
			cluster,
			info: self.dir_info.write().entry(cluster).or_default().clone(),
		}
	}

	pub fn close_file(&self, file_cluster: ClusterNum) {
		let mut lh_files = self.open_files.write();
		if lh_files.get_mut(&file_cluster).expect("close_file but not open?").sub_ref() {
			lh_files.remove(&file_cluster);
		}
	}
}
/// Directory information (mostly just the lock)
#[derive(Default,Debug)]
pub struct DirInfo
{
	lock: ::kernel::sync::RwLock<()>,
}
/// A handle to a `DirInfo` from `FilesystemInner`
struct DirInfoHandle<'a>
{
	fs: &'a FilesystemInner,
	cluster: ClusterNum,
	info: super::Arc<DirInfo>,
}
impl<'a> ::core::ops::Drop for DirInfoHandle<'a> {
	fn drop(&mut self) {
		// If the count is 2 (`self` and the `fs.dir_info` map(, then remove it from the map.
		// - Check the count before locking the map, just in case
		if super::Arc::strong_count(&self.info) == 2 {
			let mut map_lh = self.fs.dir_info.write();
			if super::Arc::strong_count(&self.info) == 2 {
				map_lh.remove(&self.cluster);
			}
		}
	}
}

pub fn update_file_size(fs: &FilesystemInner, file_cluster: ClusterNum, new_size: u32) -> Result<(), ::vfs::Error> {
	// Get the dir info, lock it, iterate the directory looking for this file

	// Challenges:
	// - Parent directory being deleted? (can't be deleted if not empty).
	// - File being moved to a different dir (potentially multiple times!)
	//
	// Solution? A map of open files, listing the relevant directory for each
	// - Has a race between lookup of that map and the dir being deleted?
	//   - Can't delete the dir if it's not empty, and can't remove the file while the map is open.
	//   - But now there's a lock ordering issue (open file map and dir lock)
	
	// Lock the file list and get the current file
	let lh_files = fs.open_files.read();
	let file_info = lh_files.get(&file_cluster).ok_or(vfs::Error::Unknown("FAT: update_file_size called with file not recorded open"))?;
	// Get/create the current directory info (shared ownership)
	let dir_info = fs.get_dir_info(file_info.dir_cluster);
	let _lh_dir = dir_info.info.lock.read();	// Entry count isn't changing, so can be a read lock

	// Iterate the dir, find the file, update
	for c in dir_clusters(fs, file_info.dir_cluster)
	{
		if let Some(found) = ::kernel::futures::block_on(fs.with_cluster(c, |cluster| {
			for (i,ent) in DirEnts::new(&cluster).enumerate()
			{
				match ent {
				DirEnt::End => return Some(None),
				DirEnt::Short(e) if e.cluster == file_cluster => return Some(Some(i)),
				_ => {},
				}
			}
			None
		}))? {
			match found
			{
			None => break,
			Some(idx) => 
				return Ok( ::kernel::futures::block_on(fs.edit_cluster(c, |cluster| {
					let data = &mut cluster[idx*32..][..32];
					let mut ent = DirEnt::from_raw(&data[..]);
					match ent {
					DirEnt::Short(ref mut e) => e.size = new_size,
					_ => unreachable!()
					}
					ent.to_raw(data);
					})).map(|_| ())? )
			}
		}
	}
	Err(vfs::Error::Unknown("FAT: update_file_size didn't find entry"))
}

fn dir_clusters(fs: &super::FilesystemInner, start_cluster: ClusterNum) -> ClusterList<'_> {
	let is_fixed_root = !is!(fs.ty, super::Size::Fat32) && start_cluster == fs.root_first_cluster;
	if is_fixed_root {
		let root_cluster_count = (fs.root_sector_count as usize + fs.spc-1) / fs.spc;
		ClusterList::range(fs.root_first_cluster, root_cluster_count as u32)
	}
	else {
		ClusterList::chained(fs, start_cluster)
	}
}

impl DirNode {
	fn clusters(&self) -> ClusterList<'_> {
		dir_clusters(&self.fs, self.start_cluster)
	}

	fn iterate_ents<T>(&self, skip: usize, mut cb: impl FnMut(usize, DirEnt)->Option<T>) -> Result<Option<T>, super::storage::IoError> {
		let ents_per_cluster = self.fs.cluster_size / 32;
		let mut idx = skip / ents_per_cluster * ents_per_cluster;
		for c in self.clusters().skip(skip / ents_per_cluster)
		{
			if let Some(rv) = ::kernel::futures::block_on(self.fs.with_cluster(c, |cluster| {
				for ent in DirEnts::new(&cluster) {
					let is_end = matches!(ent, DirEnt::End);
					if idx >= skip {
						log_trace!("{:?}", ent);
						if let Some(rv) = cb(idx, ent) {
							return Some(Some(rv));
						}
					}
					idx += 1;
					if is_end {
						return Some(None);
					}
				}
				None
			}))? {
				return Ok(rv);
			}
		}
		Ok(None)
	}

	/// Locate a node in this directory by its first data cluster
	pub fn find_node(&self, ent_cluster: ClusterNum) -> Result<Option<node::Node>, super::storage::IoError>
	{
		// Lock the file list, then the directory
		let mut lh_files = self.fs.open_files.write();
		let dir_info = self.fs.get_dir_info(self.start_cluster);
		let _lh_dir = dir_info.info.lock.read();
		log_debug!("DirNode::find_node({})", ent_cluster);
		Ok(match self.find_ent_by_cluster(ent_cluster)?
		{
		None => None,
		Some(e) =>
			if e.attributes & on_disk::ATTR_DIRECTORY != 0 {
				Some(node::Node::Dir(DirNode::new_boxed(self.fs.reborrow(), ent_cluster)))
			}
			else if e.attributes & on_disk::ATTR_VOLUMEID != 0 {
				None
			}
			else {
				lh_files.entry(ent_cluster).or_insert(OpenFileInfo::new(self.start_cluster)).add_ref();
				Some(node::Node::File(FileNode::new_boxed( self.fs.reborrow(), ent_cluster, e.size )))
			},
		})
	}
	
	fn find_ent_by_cluster(&self, ent_cluster: ClusterNum) -> Result<Option<DirEntShort>, super::storage::IoError> {
		log_trace!("find_ent_by_cluster(self={:?}, ent_cluster={})", self, ent_cluster);
		self.iterate_ents(0, |_i, ent| {
			match ent
			{
			DirEnt::Short(e) if e.cluster == ent_cluster => Some(e),
			_ => None,
			}
		})
	}
}

/// Iterator over directory entries
struct DirEnts<'a>
{
	cluster: &'a [u8],
	ofs: usize,
}

/// Directory entry returned by the DirEnts iterator
#[derive(Debug)]
enum DirEnt {
	End,
	Empty,
	Short(DirEntShort),
	Long(DirEntLong),
	Invalid(on_disk::DirEnt),
}
impl DirEnt {
	fn from_raw(slice: &[u8]) -> Self {
		// Decode the legacy format entry
		let ent = on_disk::DirEnt::read(&mut {slice});
		if ent.name[0] == 0 {
			DirEnt::End
		}
		else if ent.name[0] == b'\xE5' {
			DirEnt::Empty
		}
		else if ent.attribs == on_disk::ATTR_LFN {
			// Long filename entry
			let lfn = on_disk::DirEntLong::read(&mut {slice});
			let outname = {
				let mut outname = [0u16; 13];
				outname[0..5].clone_from_slice(&lfn.name1);
				outname[5..11].clone_from_slice(&lfn.name2);
				outname[11..13].clone_from_slice(&lfn.name3);
				outname
				};
			DirEnt::Long( DirEntLong{
				id: lfn.id,
				_type: lfn.ty,
				chars: outname,
				} )
		}
		else if ent.attribs & on_disk::ATTR_VOLUMEID != 0 {
			// TODO: I need a better value than Empty for reserved entries
			DirEnt::Invalid(ent)
		}
		else {
			// Short entry
			let lower_base = (ent.lcase & on_disk::CASE_LOWER_BASE) != 0;
			let lower_ext  = (ent.lcase & on_disk::CASE_LOWER_EXT ) != 0;
			// 1. Decode name into a NUL-padded string
			let (outname, _) = {
				let (mut outname, mut oidx) =  ([0u8; 12/*8+1+3*/], 0);
				for iidx in 0 .. 8 {
					if ent.name[iidx] != b' ' {
						outname[oidx] = ent.name[iidx];
						if lower_base {
							outname[oidx] = outname[oidx].to_ascii_lowercase();
						}
						oidx += 1;
					}
				}
				outname[oidx] = b'.';
				oidx += 1;
				for iidx in 8 .. 11 {
					if ent.name[iidx] != b' ' {
						outname[oidx] = ent.name[iidx];
						if lower_ext {
							outname[oidx] = outname[oidx].to_ascii_lowercase();
						}
						oidx += 1;
					}
				}
				if outname[oidx-1] == b'.' {
					outname[oidx-1] = 0;
					oidx -= 1;
				}
				(outname, oidx)
				};
			// 3. Cluster, Size, Attribs
			DirEnt::Short(DirEntShort{
				name: outname,
				cluster: ClusterNum::new( (ent.cluster as u32) | (ent.cluster_hi as u32) << 16 ).unwrap_or( ClusterNum::new(0xFF_FFFF).unwrap() ),
				size: ent.size,
				attributes: ent.attribs,
				})
		}
	}

	fn to_raw(&self, mut dst: &mut [u8]) {
		match self
		{
		DirEnt::End => dst.copy_from_slice(&[0; 32]),
		DirEnt::Short(v) => {
			let mut name = [b' '; 8+3];
			let dpos = v.name.iter().position(|&v| v == b'.');
			let epos = v.name.iter().position(|&v| v == b'\0').unwrap_or(8+1+3);
			let mut lcase = 0;
			for i in 0 .. dpos.unwrap_or(epos) {
				assert!(i < 8);
				if v.name[i].is_ascii_lowercase() {
					lcase |= on_disk::CASE_LOWER_BASE;
				}
				name[i] = v.name[i].to_ascii_uppercase();
			}
			if let Some(dpos) = dpos {
				for i in dpos+1 .. epos {
					if v.name[i].is_ascii_lowercase() {
						lcase |= on_disk::CASE_LOWER_EXT;
					}
					name[8 + i - (dpos+1)] = v.name[i].to_ascii_uppercase();
				}
			}
			on_disk::DirEnt {
				name,
				attribs: v.attributes,
				lcase,
				size: v.size,
				cluster: v.cluster.get() as u16,
				cluster_hi: (v.cluster.get() >> 16) as u16,
				creation_ds: 0,
				creation_date: 0,
				creation_time: 0,
				accessed_date: 0,
				modified_date: 0,
				modified_time: 0,
				}.write(&mut dst);
			},
		_ => todo!("DirEnt::to_raw: {:?}", self),
		}
	}
}
struct DirEntShort {
	/// NUL-padded string with extention joined
	name: [u8; 8+1+3],
	cluster: ClusterNum,
	size: u32,
	attributes: u8,
	//creation_time: ::kernel::time::Timestamp,
	//modified_time: ::kernel::time::Timestamp,
	//accessed_time: ::kernel::time::Timestamp,
}
impl_fmt! {
	Debug(self,f) for DirEntShort {
		write!(f, "{{ attributes: {:#x}, name: {:?}, cluster: {}, size: {:#x} }}",
			self.attributes, ByteStr::new(&self.name.split(|x|*x==0).next().unwrap()),
			self.cluster, self.size
			)
	}
}
struct DirEntLong {
	id: u8,
	_type: u8,
	chars: [u16; 13],
}
impl_fmt! {
	Debug(self,f) for DirEntLong {
		write!(f, "{{ id: {:#x}, _type: {:#x}, chars: {:?} }}",
			self.id, self._type, Str16::new(self.chars.split(|x|*x==0).next().unwrap())
			)
	}
}
impl<'a> DirEnts<'a>
{
	fn new(data: &[u8]) -> DirEnts {
		assert_eq!(data.len() % 32, 0);
		DirEnts {
			cluster: data,
			ofs: 0,
		}
	}
}
impl<'a> ::core::iter::Iterator for DirEnts<'a> {
	type Item = DirEnt;
	fn next(&mut self) -> Option<DirEnt> {
		if self.ofs >= self.cluster.len() / 32 {
			None
		}
		else {
			let slice = &self.cluster[self.ofs*32..];
			self.ofs += 1;
			Some(DirEnt::from_raw(&slice[..32]))
		}
	}
}
impl DirEntShort {
	fn name(&self) -> &ByteStr {
		ByteStr::new( (&self.name).split(|&e|e==0).next().unwrap() )
	}
	fn inode(&self, parent_dir: ClusterNum) -> node::InodeId {
		super::InodeRef::new(self.cluster, parent_dir).to_id()
	}
}

/// Decoded long file name
/// (next id, data)
struct LFN
{
	next_idx: u8,
	data: [u16; 256]
}
impl LFN {
	fn new() -> Self {
		LFN { next_idx: 0, data: [0; 256] }
	}
	fn clear(&mut self) {
		self.next_idx = 0;
		self.data[0] = 0;
	}
	fn add(&mut self, ent: &DirEntLong) {
		let idx = (ent.id & 0x3F) as usize;
		// If index is zero, this entry is invalid
		if idx == 0 {
			self.clear();
			return ;
		}
		// if 0x40 is set
		if ent.id & 0x40 != 0 {
			// - Reset state (first entry)
			self.data = [0; 256];
		}
		else {
			// Otherwise, check index is as expected
			if idx as u8 != self.next_idx {
				self.clear();
				return ;
			}
		}
		self.next_idx = (idx-1) as u8;
		let ofs = (idx-1) * 13;
		self.data[ofs..][..13].clone_from_slice( &ent.chars );
	}
	fn is_valid(&self) -> bool {
		self.next_idx == 0 && self.data[0] != 0
	}
	fn as_slice(&self) -> &[u16] {
		self.data.split(|&x| x == 0).next().unwrap()
	}
	fn name(&self) -> &Str16 {
		Str16::new(self.as_slice()).unwrap_or( Str16::new(&[]).unwrap() )
	}
}

impl node::Dir for DirNode {
	fn lookup(&self, name: &ByteStr) -> node::Result<node::InodeId> {
		log_trace!("DirNode::lookup({:?})", name);
		let dir_info = self.fs.get_dir_info(self.start_cluster);
		let _lh_dir = dir_info.info.lock.read();
		// For each cluster in the directory, iterate
		let mut lfn = LFN::new();
		match self.iterate_ents(0, |_i, ent| {
			match ent {
			DirEnt::End => {},
			DirEnt::Short(e) => {
				if e.name() == name || lfn.name() == name {
					return Some( e.inode(self.start_cluster) );
				}
				lfn.clear();
				},
			DirEnt::Long(e) => lfn.add(&e),
			DirEnt::Empty => {
				lfn.clear();
				},
			DirEnt::Invalid(_) => lfn.clear(),
			}
			None
			})?
		{
		Some(v) => Ok(v),
		None => Err(vfs::Error::NotFound)
		}
	}
	fn read(&self, ofs: usize, callback: &mut node::ReadDirCallback) -> node::Result<usize> {
		log_trace!("DirNode::read(ofs={})", ofs);
		let dir_info = self.fs.get_dir_info(self.start_cluster);
		let _lh_dir = dir_info.info.lock.read();
		
		let ents_per_cluster = self.fs.cluster_size / 32;
		let (cluster_idx, c_ofs) = (ofs / ents_per_cluster, ofs % ents_per_cluster);
		
		let mut lfn = LFN::new();
		let mut cur_ofs = ofs;
		if let Some(rv) = self.iterate_ents(ofs, |ofs, ent| {
			cur_ofs = ofs;
			match ent
			{
			// On next call, we want to hit this entry (so we can return count=0)
			DirEnt::End => return Some(cur_ofs),
			DirEnt::Short(e) => {
				let inode = e.inode(self.start_cluster);
				let cont = if lfn.is_valid() {
						callback(inode, &mut lfn.name().wtf8())
					}
					else {
						callback(inode, &mut e.name().as_bytes().iter().cloned())
					};
				if ! cont {
					return Some(cur_ofs+1);
				}
				lfn.clear();
				},
			DirEnt::Long(e) => lfn.add(&e),
			DirEnt::Empty => {
				lfn.clear();
				},
			DirEnt::Invalid(_) => lfn.clear(),
			}
			None
		})? {
			Ok(rv)
		}
		else {
			Ok( cur_ofs+1 )
		}
	}
	fn create(&self, name: &ByteStr, nodetype: node::NodeType) -> node::Result<node::InodeId> {
		// File cluster for the dir's data
		let Some(new_cluster) = self.fs.alloc_cluster_unchained(self.start_cluster)? else {
			return Err(vfs::Error::OutOfSpace);
			};
		log_debug!("DirNode::create('{:?}', {:?}): new_cluster={}", name, nodetype, new_cluster);
		fn is_valid_short_char(b: u8) -> bool {
			b.is_ascii_uppercase() || b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'
		}
		/// Check if the passed string is a valid short file name, and return the encoded version if it is
		fn is_valid_short_name(name: &ByteStr) -> Option<[u8; 8+1+3]> {
			let mut rv = [0; 8+1+3];
			let mut dotpos = None;
			let mut has_upper = false;
			let mut has_lower = false;
			for (i,&b) in name.as_bytes().iter().enumerate() {
				if b == b'.' && i > 0 {	// leading dot isn't valid
					dotpos = Some(i);
					break;
				}
				else if i == 8 {
					return None;
				}
				else if !is_valid_short_char(b) {
					return None;
				}
				else {
					rv[i] = b;
					has_lower |= b.is_ascii_lowercase();
					has_upper |= b.is_ascii_uppercase();
				}
			}
			if has_upper && has_lower {
				return None;
			}
			if let Some(dotpos) = dotpos {
				let mut has_upper = false;
				let mut has_lower = false;
				rv[dotpos] = b'.';
				for (i,&b) in name.as_bytes()[dotpos+1..].iter().enumerate() {
					if i == 3 {
						return None;
					}
					else if !is_valid_short_char(b) {
						return None;
					}
					else {
						rv[dotpos+1+i] = b;
						has_lower |= b.is_ascii_lowercase();
						has_upper |= b.is_ascii_uppercase();
					}
				}
				if has_upper && has_lower {
					return None;
				}
			}
			Some(rv)
		}
		fn make_short_name(name: &ByteStr, index: u16) -> [u8; 8+1+3] {
			let mut rv = [0; 8+1+3];
			let mut i = 0;
			// Strip leading invalid characters
			let mut iter = name.as_bytes().iter().copied().peekable();
			if iter.peek() == Some(&b'.') {
				iter.next();
			}
			let mut has_ext = false;
			while let Some(v) = iter.next() {
				if i > 0 && v == b'.' {
					has_ext = true;
					break;
				}
				if i == 8 {
					rv[6] = b'~';
					rv[7] = b'1';
					break;
				}
				if is_valid_short_char(v.to_ascii_uppercase()) {
					rv[i] = v.to_ascii_uppercase();
					i += 1;
				}
			}
			if i == 0 {
				rv[i] = b'_';
				i += 1;
			}
			if has_ext && iter.peek().is_some() {
				rv[i] = b'.';
				i += 1;
				while let Some(v) = iter.next() {
					if is_valid_short_char(v.to_ascii_uppercase()) {
						rv[i] = v.to_ascii_uppercase();
						i += 1;
					}
				}
				if rv[i-1] == b'.' {
					rv[i] = b'_';
				}
			}

			if index > 0 {
				todo!("Mangle short name for de-duplication");
			}

			rv
		}
		let short_name;
		// Add entries to the end of the dir
		// - Determine if this file can be encoded as a short filename, and if not - how many entries it will need
		let num_entries = if let Some(sn) = is_valid_short_name(name) {
			short_name = sn;
			1
		} else {
			short_name = make_short_name(name, 0);
			1 + (::utf16::wtf8_to_utf16(name.as_bytes()).count() + 1 + 13-1) / 13
		};

		let dir_info = self.fs.get_dir_info(self.start_cluster);
		let _lh_dir = dir_info.info.lock.write();
		// - Lock the directory, then start seeking clusters looking for a sequence of slots large enough
		let mut end_entry = None;
		let (found_slot, short_collision, total_free_slots, end_idx) = {
			let mut found_slot = None;
			let mut short_collision = false;
			let mut n_free_total = 0;
			let mut n_free_run = 0;	// Number of free entries in a row currently seen
			let mut lfn = LFN::new();
			let mut idx = 0;	// Index of the directory entry
			// TODO: `iterate_ent` doesn't return the cluster - so reimplemented here
			for c in self.clusters()
			{
				if let Some(rv) = ::kernel::futures::block_on(self.fs.with_cluster(c, |cluster| {
					for (i,ent) in DirEnts::new(&cluster).enumerate()
					{
						match ent {
						DirEnt::End => {
							end_entry = Some( (c,i) );
							break ;
							},
						DirEnt::Short(ref e) => {
							if e.name() == name || lfn.name() == name {
								return Some(Err(vfs::Error::AlreadyExists));
							}
							if e.name().as_bytes() == &short_name {
								short_collision = true;
							}
							lfn.clear();
							},
						DirEnt::Long(ref e) => lfn.add(e),
						DirEnt::Empty => {
							lfn.clear();
							},
						DirEnt::Invalid(_) => lfn.clear(),
						}
						if let DirEnt::Empty = ent {
							n_free_run += 1;
							n_free_total += 1;
							if found_slot.is_none() && n_free_run == num_entries {
								found_slot = Some(idx);
							}
						}
						else {
							n_free_run = 0;
						}
						idx += 1;
					}
					None
					}))? {
					return rv;
				}
				if end_entry.is_some() {
					break;
				}
			}
			(found_slot, short_collision, n_free_total, idx)
		};

		// If there was a short name collision (... which can only be flagged if LFN is used, otherwise it's an error)
		// then re-iterate and create a list of short names
		if short_collision
		{
			let mut names = vec![];
			self.iterate_ents(0, |_i, ent| {
				if let DirEnt::Short(e) = ent {
					// TODO: Only push if it prefix matches the preferred name (until the `~`)
					names.push( e.name );
				}
				None::<()>
			})?;
			todo!("DirNode::create(): Handle short name collision");
		}

		let mut ents_it = CreateDirents::new(nodetype, new_cluster, short_name, if num_entries == 1 { None } else {Some(name) })
			.chain(::core::iter::once(DirEnt::End))
			.peekable()
			;

		if let Some(pos) = found_slot {
			// Can freely insert
			todo!("DirNode::create('{:?}', {:?}): new_cluster={}, num_entries={} - found_slot={:?}", name, nodetype, new_cluster, num_entries, found_slot);
		}
		else {
			// Extend the directory
			// - Should this also defragment?
			if total_free_slots >= num_entries {
				// There were enough entries - need to defragment the directory
				todo!("DirNode::create(): Defragment directory ({} total free, needed {} contig)", total_free_slots, num_entries);
			}
			else {
				// Don't bother defragmenting, just extend the length
				// - Maybe the existing last cluster can be extended...

				// NOTE: No need to update the size, as the size of a directory is always zero.

				let mut clusters_it = self.clusters();
				let mut cluster = self.start_cluster;
				let mut idx = self.fs.cluster_size / 32;	// Default to past the end
				while let Some(c) = clusters_it.next() {
					cluster = c;
					if let Some((end_cluster, end_idx)) = end_entry {
						if c == end_cluster {
							idx = end_idx;
							break;
						}
					}
				}

				while ents_it.peek().is_some()
				{
					assert!(idx <= self.fs.cluster_size / 32);
					if idx == self.fs.cluster_size / 32 {
						if let Some(next_cluster) = clusters_it.next() {
							cluster = next_cluster;
						}
						else {
							todo!("Allocate a new cluster?");
						}
						idx = 0;
					}
					::kernel::futures::block_on(self.fs.edit_cluster(cluster, |data| {
						while idx < self.fs.cluster_size / 32 {
							let Some(ent) = ents_it.next() else { break; };
							log_debug!("WRITE: {} @ {} {:?}", cluster, idx*32, ent);
							ent.to_raw(&mut data[idx*32..][..32]);
							log_debug!("- {:x?}", &data[idx*32..][..32]);
							idx += 1;
						}
						}))?;
				}
			}
		}
		Ok( super::InodeRef::new(new_cluster, self.start_cluster).to_id() )
	}
	fn link(&self, name: &ByteStr, node: &dyn node::NodeBase) -> node::Result<()> {
		todo!("DirNode::link('{:?}', {:#x})", name, node.get_id());
	}
	fn unlink(&self, name: &ByteStr) -> node::Result<()> {
		todo!("DirNode::unlink('{:?}')", name);
	}
}


// ---
//
// ---
struct CreateDirents {
	real_ent: Option<DirEntShort>,
	long_name: ::core::iter::Rev<::kernel::lib::vec::IntoIter<DirEntLong>>,
}
impl CreateDirents {
	fn new(node_type: node::NodeType, target_cluster: ClusterNum, name: [u8; 8+1+3], long_name: Option<&'_ ByteStr>) -> Self {
		CreateDirents {
			real_ent: Some(DirEntShort {
				name,
				cluster: target_cluster,
				size: 0,
				attributes: match node_type
					{
					node::NodeType::File => on_disk::ATTR_ARCHIVE,
					node::NodeType::Dir => on_disk::ATTR_DIRECTORY,
					node::NodeType::Symlink(_) => todo!("Symlink"),
					},
				}),
			long_name: {
				let mut it = ::utf16::wtf8_to_utf16(long_name.map(|v| v.as_bytes()).unwrap_or(&[]));
				let mut rv = vec![];
				while let Some(cp) = it.next() {
					let mut seg = [0u16; 13];
					seg[0] = cp;
					for i in 1 .. 13 {
						let Some(cp) = it.next() else { break; };
						seg[i] = cp;
					}
					rv.push(DirEntLong {
						id: 1 + rv.len() as u8,
						_type: 0,	// TODO: What should this be?
						chars: seg
					});
				}
				if let Some(v) = rv.last_mut() {
					v.id |= 0x40;
				}
				rv.into_iter().rev()
				}
		}
	}
}
impl ::core::iter::Iterator for CreateDirents {
	type Item = DirEnt;
	fn next(&mut self) -> Option<DirEnt> {
		if let Some(v) = self.long_name.next() {
			return Some(DirEnt::Long(v));
		}
		if let Some(v) = self.real_ent.take() {
			return Some(DirEnt::Short(v));
		}
		None
	}
}
