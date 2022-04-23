// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/fs_fat/dir.rs
use kernel::prelude::*;
use kernel::lib::mem::aref::ArefBorrow;
use kernel::vfs::{self, node};
use super::FilesystemInner;

const ERROR_SHORTCHAIN: vfs::Error = vfs::Error::Unknown("Cluster chain terminated early");

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
	fn get_any(&self) -> &dyn core::any::Any {
		self
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
		if ofs > self.size as u64 {
			// out of range
			return Err( vfs::Error::InvalidParameter );
		}
		if ofs == self.size as u64 {
			return Ok(0);
		}
		let maxread = (self.size as u64 - ofs) as usize;
		let buf = if buf.len() > maxread { &mut buf[..maxread] } else { buf };
		let read_length = buf.len();
		
		// Seek to correct position in the cluster chain
		let mut clusters = super::ClusterList::chained(self.fs.reborrow(), self.first_cluster);
		for _ in 0 .. (ofs/self.fs.cluster_size as u64) {
			clusters.next();
		}
		let ofs = (ofs % self.fs.cluster_size as u64) as usize;
		
		// First incomplete cluster
		let mut cur_read_ofs = 0;
		/*let chunks = */if ofs != 0 {
				let cluster = match clusters.next()
					{
					Some(v) => v,
					None => return Err( ERROR_SHORTCHAIN ),
					};
				let short_count = ::core::cmp::min(self.fs.cluster_size-ofs, buf.len());
				let c = self.fs.load_cluster(cluster)?;
				buf[..short_count].clone_from_slice( &c[ofs..][..short_count] );
				
				cur_read_ofs += short_count;
				//buf[short_count..].chunks_mut(self.fs.cluster_size)
			}
			else {
				//buf.chunks_mut(self.fs.cluster_size)
			};
	
		//#[cfg(DISABLED)]
		while buf.len() - cur_read_ofs >= self.fs.cluster_size
		{
			let dst = &mut buf[cur_read_ofs..];
			let (cluster, count) = match clusters.next_extent( dst.len() / self.fs.cluster_size )
				{
				Some(v) => v,
				None => {
					log_notice!("Unexpected end of cluster chain at offset {}", cur_read_ofs);
					return Err(ERROR_SHORTCHAIN);
					},
				};
			let bytes = count * self.fs.cluster_size;
			log_trace!("- Read cluster {}+{}", cluster, count);
			::kernel::futures::block_on(self.fs.read_clusters(cluster, &mut dst[..bytes]))?;
			cur_read_ofs += bytes;
		}
		//#[cfg(DISABLED)]
		if buf.len() - cur_read_ofs > 0
		{
			let dst = &mut buf[cur_read_ofs..];
			let cluster = match clusters.next()
				{
				Some(v) => v,
				None => {
					log_notice!("Unexpected end of cluster chain at offset {}", cur_read_ofs);
					return Err(ERROR_SHORTCHAIN);
					},
				};
			let c = self.fs.load_cluster(cluster)?;
			let bytes = dst.len();
			dst.clone_from_slice( &c[..bytes] );
		}

		// The rest of the clusters
		/*
		for dst in chunks
		{
			// TODO: Cluster extents
			let cluster = match clusters.next()
				{
				Some(v) => v,
				None => {
					log_notice!("Unexpected end of cluster chain at offset {}", cur_read_ofs);
					return Err(ERROR_SHORTCHAIN);
					},
				};
			if dst.len() == self.fs.cluster_size {
				// Read directly
				try!(self.fs.read_cluster(cluster, dst));
			}
			else {
				// Bounce (could leave the bouncing up to read_cluster I guess...)
				let c = try!(self.fs.load_cluster(cluster));
				let bytes = dst.len();
				dst.clone_from_slice( &c[..bytes] );
			}
			cur_read_ofs += dst.len();
		}
		*/
		
		Ok( read_length )
	}
	/// Write data to the file, can only grow the file if ofs==size
	fn write(&self, ofs: u64, buf: &[u8]) -> node::Result<usize> {
		todo!("FileNode::write({:#x}, {:p})", ofs, ::kernel::lib::SlicePtr(buf));
	}
}

