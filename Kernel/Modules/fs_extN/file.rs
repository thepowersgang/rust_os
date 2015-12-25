// "Tifflin" Kernel - ext2/3/4 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_extN/file.rs
//! Regular file
use kernel::prelude::*;
use kernel::vfs;

pub struct File
{
	inode: ::inodes::Inode,
}


impl File
{
	pub fn new(inode: ::inodes::Inode) -> File
	{
		File {
			inode: inode,
			}
	}

	fn fs_block_size(&self) -> usize {
		self.inode.fs.fs_block_size
	}
}

impl vfs::node::NodeBase for File
{
	fn get_id(&self) -> vfs::node::InodeId {
		self.inode.get_id()
	}
}
impl vfs::node::File for File
{
	fn size(&self) -> u64 {
		self.inode.i_size()
	}
	fn read(&self, ofs: u64, buf: &mut [u8]) -> vfs::node::Result<usize>
	{
		// 1. Restrict buffer size to avaiable bytes
		let avail_bytes = self.inode.i_size() - ofs;
		let buf = if buf.len() as u64 > avail_bytes {
				&mut buf[.. avail_bytes as usize]
			}
			else {
				buf
			};

		// 2. Get first block and offset into that block
		let (blk_idx, blk_ofs) = ::kernel::lib::num::div_rem(ofs, self.fs_block_size() as u64);
		let blk_ofs = blk_ofs as usize;

		assert!(blk_idx <= ::core::u32::MAX as u64);
		let mut blocks = self.inode.blocks_from(blk_idx as u32);
		let mut read_bytes = 0;

		// 3. Read leading partial block
		if blk_ofs != 0
		{
			let partial_bytes = self.fs_block_size() - blk_ofs;
			
			let blk_data = try!(self.inode.fs.get_block( try!(blocks.next_or_err()) ));
			let blk_data = ::kernel::lib::as_byte_slice(&blk_data);
			if buf.len() <= partial_bytes
			{
				buf.clone_from_slice( blk_data );
				read_bytes += buf.len();
			}
			else
			{
				buf[..partial_bytes].clone_from_slice(blk_data);
				read_bytes += partial_bytes;
			}
		}

		// 4. Read full blocks
		if buf.len() - read_bytes >= self.fs_block_size()
		{
			let blkid = try!(blocks.next_or_err());
			try!(self.inode.fs.read_blocks(blkid, &mut buf[read_bytes ..][.. self.fs_block_size()]));
		}

		// 5. Read the trailing partial block
		if buf.len() - read_bytes > 0
		{
			let blk_data = try!(self.inode.fs.get_block( try!(blocks.next_or_err()) ));
			let blk_data = ::kernel::lib::as_byte_slice(&blk_data);
			buf[read_bytes..].clone_from_slice(&blk_data);
			read_bytes += buf.len();
		}

		// 6. Return number of bytes read (which may be smaller than the original buffer length)
		Ok( read_bytes )
	}

	fn truncate(&self, newsize: u64) -> vfs::node::Result<u64> {
		todo!("truncate");
	}
	fn clear(&self, ofs: u64, size: u64) -> vfs::node::Result<()> {
		todo!("clear");
	}
	fn write(&self, ofs: u64, buf: &mut [u8]) -> vfs::node::Result<usize> {
		todo!("write");
	}
}

