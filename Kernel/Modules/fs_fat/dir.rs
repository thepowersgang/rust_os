// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/fs_fat/dir.rs
use kernel::prelude::*;
use kernel::lib::mem::aref::ArefBorrow;
use kernel::vfs::node;
use kernel::lib::byte_str::{ByteStr,ByteString};
use super::on_disk;
use super::file::FileNode;
use super::ClusterList;

pub type FilesystemInner = super::FilesystemInner;

pub struct DirNode
{
	fs: ArefBorrow<FilesystemInner>,
	start_cluster: u32,
	// - Uses the cluster chain
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
		todo!("")
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
#[derive(Debug)]
struct DirEntShort {
	/// NUL-padded string with extention joined
	name: [u8; 11+1],
	cluster: u32,
	size: u32,
	attributes: u8,
	//creation_time: ::kernel::time::Timestamp,
	//modified_time: ::kernel::time::Timestamp,
	//accessed_time: ::kernel::time::Timestamp,
}
#[derive(Debug)]
struct DirEntLong {
	id: u8,
	_type: u8,
	chars: [u16; 13],
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
			else {
				// Short entry
				// 1. Decode name into a NUL-padded string
				let (outname, _) = {
					let (mut outname, mut oidx) =  ([0u8; 8+1+3], 0);
					for iidx in (0 .. 8) {
						if ent.name[iidx] != b' ' {
							outname[oidx] = ent.name[iidx];
							oidx += 1;
						}
					}
					outname[oidx] = b'.';
					oidx += 1;
					for iidx in (8 .. 11) {
						if ent.name[iidx] != b' ' {
							outname[oidx] = ent.name[iidx];
							oidx += 1;
						}
					}
					if outname[oidx-1] == b'.' {
						outname[oidx-1] = 0;
						oidx -= 1;
					}
					(outname, oidx)
					};
				// 2. Cluster, Size, Attribs
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

impl node::Dir for DirNode {
	fn lookup(&self, name: &ByteStr) -> node::Result<node::InodeId> {
		// For each cluster in the directory, iterate
		for c in self.clusters()
		{
			let cluster = try!(self.fs.load_cluster(c));
			for ent in DirEnts::new(&cluster) {
				log_debug!("ent = {:?}", ent);
				match ent {
				DirEnt::End => return Err(node::IoError::NotFound),
				DirEnt::Short(e) =>
					if e.name() == name {
						return Ok( e.inode(self.start_cluster) );
					},
				DirEnt::Long(_) => log_log!("TODO: LFN"),
				DirEnt::Empty => {},
				}
			}
		}
		Err(node::IoError::NotFound)
	}
	fn read(&self, ofs: usize, items: &mut [(node::InodeId,ByteString)]) -> node::Result<(usize,usize)> {
		
		let ents_per_cluster = self.fs.cluster_size / 32;
		let (cluster_idx, c_ofs) = (ofs / ents_per_cluster, ofs % ents_per_cluster);
		
		let mut count = 0;
		let mut cur_ofs = ofs;
		'outer: for c in self.clusters().skip(cluster_idx)
		{
			let cluster = try!(self.fs.load_cluster(c));
			for ent in DirEnts::new(&cluster).skip(c_ofs) {
				cur_ofs += 1;
				match ent {
				DirEnt::End => {
					// On next call, we want to hit this entry (so we can return count=0)
					return Ok( (cur_ofs-1, count) );
					},
				DirEnt::Short(e) => {
					items[count] = (e.inode(self.start_cluster), ByteString::from(e.name()));
					count += 1;
					if count == items.len() {
						return Ok( (cur_ofs, count) );
					}
					},
				DirEnt::Long(_) => log_log!("TODO: LFN"),
				DirEnt::Empty => {},
				}
			}
		}
		
		Ok( (cur_ofs, count) )
	}
	fn create(&self, name: &ByteStr, nodetype: node::NodeType) -> node::Result<node::InodeId> {
		todo!("DirNode::create('{:?}', {:?})", name, nodetype);
	}
	fn link(&self, name: &ByteStr, inode: node::InodeId) -> node::Result<()> {
		todo!("DirNode::link('{:?}', {:#x})", name, inode);
	}
	fn unlink(&self, name: &ByteStr) -> node::Result<()> {
		todo!("DirNode::unlink('{:?}')", name);
	}
}

