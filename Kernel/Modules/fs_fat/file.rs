// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Modules/fs_fat/dir.rs
use kernel::prelude::*;
use kernel::lib::mem::aref::ArefBorrow;
use kernel::vfs::{self, node};
use super::FilesystemInner;
use super::ClusterNum;

const ERROR_SHORTCHAIN: vfs::Error = vfs::Error::Unknown("Cluster chain terminated early");

pub struct FileNode
{
	fs: ArefBorrow<FilesystemInner>,
	first_cluster: ClusterNum,
	size: ::kernel::sync::RwLock<u32>,
}

impl FileNode
{
	pub fn new_boxed(fs: ArefBorrow<FilesystemInner>, first_cluster: ClusterNum, size: u32) -> Box<FileNode> {
		Box::new(FileNode {
			fs: fs,
			first_cluster,
			size: ::kernel::sync::RwLock::new(size),
			})
	}
}
impl ::core::ops::Drop for FileNode {
	fn drop(&mut self) {
		//super::dir::close_file(&self.fs, self.first_cluster);
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
		*self.size.read() as u64
	}
	fn truncate(&self, newsize: u64) -> node::Result<u64> {
		let newsize: u32 = ::core::convert::TryFrom::try_from(newsize).unwrap_or(!0);
		let mut size_lh = self.size.write();
		if newsize < *size_lh {
			// Update size, and then deallocate clusters
			let old_size = *size_lh;
			super::dir::update_file_size(&self.fs, self.first_cluster, newsize)?;
			*size_lh = newsize;
			todo!("FileNode::truncate({:#x})", newsize);
		}
		else {
			// Allocate new clusters, then update the size
			// Update size iteratively, allocating clusters as needed
			todo!("FileNode::truncate({:#x})", newsize);
		}
	}
	fn clear(&self, ofs: u64, size: u64) -> node::Result<()> {
		todo!("FileNode::clear({:#x}+{:#x}", ofs, size);
	}
	fn read(&self, ofs: u64, buf: &mut [u8]) -> node::Result<usize> {
		let maxread = {
			let size_lh = self.size.read();
			// Sanity check and bound parameters
			if ofs > *size_lh as u64 {
				// out of range
				return Err( vfs::Error::InvalidParameter );
			}
			if ofs == *size_lh as u64 {
				return Ok(0);
			}
			(*size_lh as u64 - ofs) as usize
			};
		let buf = if buf.len() > maxread { &mut buf[..maxread] } else { buf };
		let read_length = buf.len();
		log_trace!("read(@{:#x} len={:?})", ofs, read_length);
		
		// Seek to correct position in the cluster chain
		let mut clusters = super::ClusterList::chained(&self.fs, self.first_cluster);
		for _ in 0 .. (ofs/self.fs.cluster_size as u64) {
			clusters.next();
		}
		let ofs = (ofs % self.fs.cluster_size as u64) as usize;
		
		// First incomplete cluster
		let mut cur_read_ofs = 0;
		/*let chunks = */if ofs != 0 {
				let Some(cluster) = clusters.next() else { return Err(ERROR_SHORTCHAIN); };
				let short_count = ::core::cmp::min(self.fs.cluster_size-ofs, buf.len());
				log_trace!("read(): Read partial head {} len={}", cluster, short_count);
				::kernel::futures::block_on(self.fs.with_cluster(cluster, |c| {
					buf[..short_count].clone_from_slice( &c[ofs..][..short_count] );
					}))?;
				
				cur_read_ofs += short_count;
				//buf[short_count..].chunks_mut(self.fs.cluster_size)
			}
			else {
				//buf.chunks_mut(self.fs.cluster_size)
			};

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
			log_trace!("read(): Read cluster extent {} + {}", cluster, count);
			::kernel::futures::block_on(self.fs.read_clusters(cluster, &mut dst[..bytes]))?;
			cur_read_ofs += bytes;
		}

		// Trailing sub-cluster data
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
			log_trace!("read(): Read partial tail {} len={}", cluster, dst.len());
			::kernel::futures::block_on(self.fs.with_cluster(cluster, |c| {
				let bytes = dst.len();
				dst.clone_from_slice( &c[..bytes] );
			}))?
		}

		log_trace!("read(): Complete {}", read_length);
		Ok( read_length )
	}
	/// Write data to the file, can only grow the file if ofs==size
	fn write(&self, ofs: u64, mut buf: &[u8]) -> node::Result<usize> {
		let mut size_lh = self.size.write();
		if ofs == *size_lh as u64 {
			let rv = buf.len();
			// Write data, allocating new clusters if needed
			let mut clusters = super::ClusterList::chained(&self.fs, self.first_cluster);
			let mut prev_cluster = self.first_cluster;
			for _ in 0 .. (ofs/self.fs.cluster_size as u64) {
				prev_cluster = clusters.next().ok_or(vfs::Error::InconsistentFilesystem)?;
			}
			let ofs = (ofs % self.fs.cluster_size as u64) as usize;
			while buf.len() > 0
			{
				let cluster = if let Some(c) = clusters.next() {
						c
					} else {
						// Need to allocate a new one!
						self.fs.alloc_cluster_chained(prev_cluster)?.ok_or(vfs::Error::OutOfSpace)?
					};
				let len = usize::min(self.fs.cluster_size - ofs, buf.len());
				if len == self.fs.cluster_size {
					assert!(ofs == 0);
					::kernel::futures::block_on(self.fs.write_clusters(cluster, &buf[..len]))?;
				}
				else {
					::kernel::futures::block_on(self.fs.edit_cluster(cluster, |c| {
						c[ofs..][..len].copy_from_slice(&buf[..len]);
						}))?;
				}
				buf = &buf[len..];
				*size_lh += len as u32;
			}
			// Update the size
			super::dir::update_file_size(&self.fs, self.first_cluster, *size_lh)?;

			Ok(rv)
		}
		else {
			// Don't want the size to reduce while this is happening...
			// - But it _could_ increase due to append?
			// - or just prevent a change
			todo!("FileNode::write({:#x}, {:p}) - inplace", ofs, ::kernel::lib::SlicePtr(buf));
		}
	}
}

