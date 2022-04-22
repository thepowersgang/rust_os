// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/fs_fat/dir.rs
use kernel::prelude::*;
use kernel::lib::mem::aref::ArefBorrow;
use kernel::vfs::{self, node};
use kernel::lib::byte_str::ByteStr;
use super::on_disk;
use super::file::FileNode;
use super::ClusterList;
use super::FilesystemInner;
use utf16::Str16;

pub struct DirNode
{
	fs: ArefBorrow<crate::FilesystemInner>,
	start_cluster: u32,
	// - Uses the cluster chain
}
impl_fmt! {
	Debug(self, f) for DirNode {
		write!(f, "{{cluster={:#x}}}", self.start_cluster)
	}
}

impl DirNode {
	pub fn new(fs: ArefBorrow<FilesystemInner>, start_cluster: u32) -> DirNode {
		DirNode {
			fs: fs,
			start_cluster: start_cluster,
		}
	}
	pub fn new_boxed(fs: ArefBorrow<FilesystemInner>, start_cluster: u32) -> Box<DirNode> {
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

impl DirNode {
	fn is_fixed_root(&self) -> bool {
		!is!(self.fs.ty, super::Size::Fat32) && self.start_cluster == self.fs.root_first_cluster
	}
	fn clusters(&self) -> ClusterList {
		if self.is_fixed_root() {
			let root_cluster_count = (self.fs.root_sector_count as usize + self.fs.spc-1) / self.fs.spc;
			ClusterList::Range(self.fs.root_first_cluster .. self.fs.root_first_cluster + root_cluster_count as u32)
		}
		else {
			ClusterList::Chained(self.fs.reborrow(), self.start_cluster)
		}
	}

	/// Locate a node in this directory by its first data cluster
	pub fn find_node(&self, ent_cluster: u32) -> Option<node::Node>
	{
		match self.find_ent_by_cluster(ent_cluster)
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
				Some(node::Node::File(FileNode::new_boxed(
					self.fs.reborrow(), self.start_cluster, ent_cluster, e.size
					)))
			},
		}
	}
	
	fn find_ent_by_cluster(&self, ent_cluster: u32) -> Option<DirEntShort> {
		log_trace!("find_ent_by_cluster(self={:?}, ent_cluster={})", self, ent_cluster);
		for c in self.clusters()
		{
			let cluster = match self.fs.load_cluster(c) {
				Ok(v) => v,
				Err(_) => return None,
				};
			for ent in DirEnts::new(&cluster) {
				if let DirEnt::Short(e) = ent {
					if e.cluster == ent_cluster {
						return Some(e);
					}
				}
			}
		}
		None
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
}
struct DirEntShort {
	/// NUL-padded string with extention joined
	name: [u8; 8+1+3],
	cluster: u32,
	size: u32,
	attributes: u8,
	//creation_time: ::kernel::time::Timestamp,
	//modified_time: ::kernel::time::Timestamp,
	//accessed_time: ::kernel::time::Timestamp,
}
impl_fmt! {
	Debug(self,f) for DirEntShort {
		write!(f, "{{ attributes: {:#x}, name: {:?}, cluster: {:#x}, size: {:#x} }}",
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
			// Decode the legacy format entry
			let ent = on_disk::DirEnt::read(&mut &slice[..]);
			if ent.name[0] == 0 {
				Some(DirEnt::End)
			}
			else if ent.name[0] == b'\xE5' {
				Some(DirEnt::Empty)
			}
			else if ent.attribs == on_disk::ATTR_LFN {
				// Long filename entry
				let lfn = on_disk::DirEntLong::read(&mut &slice[..]);
				let outname = {
					let mut outname = [0u16; 13];
					outname[0..5].clone_from_slice(&lfn.name1);
					outname[5..11].clone_from_slice(&lfn.name2);
					outname[11..13].clone_from_slice(&lfn.name3);
					outname
					};
				Some(DirEnt::Long( DirEntLong{
					id: lfn.id,
					_type: lfn.ty,
					chars: outname,
					} ))
			}
			else if ent.attribs & on_disk::ATTR_VOLUMEID != 0 {
				// TODO: I need a better value than Empty for reserved entries
				Some(DirEnt::Empty)
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
				Some( DirEnt::Short(DirEntShort{
					name: outname,
					cluster: (ent.cluster as u32) | (ent.cluster_hi as u32) << 16,
					size: ent.size,
					attributes: ent.attribs,
					}) )
			}
		}
	}
}
impl DirEntShort {
	fn name(&self) -> &ByteStr {
		ByteStr::new( (&self.name).split(|&e|e==0).next().unwrap() )
	}
	fn inode(&self, parent_dir: u32) -> node::InodeId {
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
		// For each cluster in the directory, iterate
		let mut lfn = LFN::new();
		for c in self.clusters()
		{
			let cluster = self.fs.load_cluster(c)?;
			for ent in DirEnts::new(&cluster)
			{
				match ent {
				DirEnt::End => return Err(vfs::Error::NotFound),
				DirEnt::Short(e) => {
					if e.name() == name || lfn.name() == name {
						return Ok( e.inode(self.start_cluster) );
					}
					lfn.clear();
					},
				DirEnt::Long(e) => lfn.add(&e),
				DirEnt::Empty => {
					lfn.clear();
					},
				}
			}
		}
		Err(vfs::Error::NotFound)
	}
	fn read(&self, ofs: usize, callback: &mut node::ReadDirCallback) -> node::Result<usize> {
		
		let ents_per_cluster = self.fs.cluster_size / 32;
		let (cluster_idx, c_ofs) = (ofs / ents_per_cluster, ofs % ents_per_cluster);
		
		let mut lfn = LFN::new();
		let mut cur_ofs = ofs;
		for c in self.clusters().skip(cluster_idx)
		{
			let cluster = self.fs.load_cluster(c)?;
			for ent in DirEnts::new(&cluster).skip(c_ofs)
			{
				cur_ofs += 1;
				match ent
				{
				DirEnt::End => {
					// On next call, we want to hit this entry (so we can return count=0)
					return Ok(cur_ofs - 1);
					},
				DirEnt::Short(e) => {
					let inode = e.inode(self.start_cluster);
					let cont = if lfn.is_valid() {
							callback(inode, &mut lfn.name().wtf8())
						}
						else {
							callback(inode, &mut e.name().as_bytes().iter().cloned())
						};
					if ! cont {
						return Ok(cur_ofs);
					}
					lfn.clear();
					},
				DirEnt::Long(e) => lfn.add(&e),
				DirEnt::Empty => {
					lfn.clear();
					},
				}
			}
		}
		
		Ok( cur_ofs )
	}
	fn create(&self, name: &ByteStr, nodetype: node::NodeType) -> node::Result<node::InodeId> {
		// File cluster for the dir's data
		let new_cluster = self.fs.alloc_cluster( self.start_cluster )?;
		// Add entries to the end of the dir
		todo!("DirNode::create('{:?}', {:?}): dir_cluster={:#x}", name, nodetype, new_cluster);
	}
	fn link(&self, name: &ByteStr, node: &dyn node::NodeBase) -> node::Result<()> {
		todo!("DirNode::link('{:?}', {:#x})", name, node.get_id());
	}
	fn unlink(&self, name: &ByteStr) -> node::Result<()> {
		todo!("DirNode::unlink('{:?}')", name);
	}
}

