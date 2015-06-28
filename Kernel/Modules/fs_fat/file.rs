// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/fs_fat/dir.rs
use kernel::prelude::*;
use kernel::lib::mem::aref::ArefBorrow;
use kernel::vfs::node;

const ERROR_SHORTCHAIN: node::IoError = node::IoError::Unknown("Cluster chain terminated early");

pub type FilesystemInner = super::FilesystemInner;

pub struct FileNode
{
	fs: ArefBorrow<FilesystemInner>,
	//parent_dir: u32,
	first_cluster: u32,
	size: u32,
}

impl FileNode
{
	pub fn new_boxed(fs: ArefBorrow<FilesystemInner>, _parent: u32, first_cluster: u32, size: u32) -> Box<FileNode> {	
		Box::new(FileNode {
			fs: fs,
			//parent_dir: parent,
			first_cluster: first_cluster,
			size: size,
			})
	}
}
impl node::NodeBase for FileNode {
	fn get_id(&self) -> node::InodeId {
		todo!("FileNode::get_id")
	}
}
impl node::File for FileNode {
	fn size(&self) -> u64 {
		self.size as u64
	}
	fn truncate(&self, newsize: u64) -> node::Result<u64> {
		todo!("FileNode::truncate({:#x})", newsize);
	}
	fn clear(&self, ofs: u64, size: u64) -> node::Result<()> {
		todo!("FileNode::clear({:#x}+{:#x}", ofs, size);
	}
	fn read(&self, ofs: u64, buf: &mut [u8]) -> node::Result<usize> {
		// Sanity check and bound parameters
		if ofs >= self.size as u64 {
			// out of range
			return Err( node::IoError::OutOfRange );
		}
		let maxread = (self.size as u64 - ofs) as usize;
		let buf = if buf.len() > maxread { &mut buf[..maxread] } else { buf };
		let read_length = buf.len();
		
		// Seek to correct position in the cluster chain
		let mut clusters = super::ClusterList::chained(self.fs.reborrow(), self.first_cluster)
			.skip( (ofs/self.fs.cluster_size as u64) as usize);
		let ofs = (ofs % self.fs.cluster_size as u64) as usize;
		
		// First incomplete cluster
		let chunks = if ofs != 0 {
				let cluster = match clusters.next()
					{
					Some(v) => v,
					None => return Err( ERROR_SHORTCHAIN ),
					};
				let short_count = ::core::cmp::min(self.fs.cluster_size-ofs, buf.len());
				let c = try!(self.fs.load_cluster(cluster));
				let n = buf[..short_count].clone_from_slice( &c[ofs..] );
				assert_eq!(n, short_count);
				
				buf[short_count..].chunks_mut(self.fs.cluster_size)
			}
			else {
				buf.chunks_mut(self.fs.cluster_size)
			};
		
		// The rest of the clusters
		for dst in chunks
		{
			let cluster = match clusters.next()
				{
				Some(v) => v,
				None => return Err(ERROR_SHORTCHAIN),
				};
			if dst.len() == self.fs.cluster_size {
				// Read directly
				try!(self.fs.read_cluster(cluster, dst));
			}
			else {
				// Bounce (could leave the bouncing up to read_cluster I guess...)
				let c = try!(self.fs.load_cluster(cluster));
				let n = dst.clone_from_slice( &c );
				assert_eq!(n, dst.len());
			}
		}
		
		Ok( read_length )
	}
	/// Write data to the file, can only grow the file if ofs==size
	fn write(&self, ofs: u64, buf: &mut [u8]) -> node::Result<usize> {
		todo!("FileNode::write({:#x}, {:p})", ofs, ::kernel::lib::SlicePtr(buf));
	}
}

