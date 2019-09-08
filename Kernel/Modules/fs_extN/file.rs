// "Tifflin" Kernel - ext2/3/4 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_extN/file.rs
//! Regular file
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
	fn get_any(&self) -> &dyn (::core::any::Any) {
		self
	}
}
impl vfs::node::File for File
{
	fn size(&self) -> u64 {
		self.inode.i_size()
	}
	fn read(&self, ofs: u64, buf: &mut [u8]) -> vfs::node::Result<usize>
	{
		if ofs > self.inode.i_size() {
			return Err(vfs::Error::InvalidParameter);
		}
		if ofs == self.inode.i_size() {
			return Ok(0);
		}
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
		//log_trace!("blk_ofs={} (partial)", blk_ofs);
		if blk_ofs != 0
		{
			let partial_bytes = self.fs_block_size() - blk_ofs;
			
			let blk_data = try!(self.inode.fs.get_block_uncached( try!(blocks.next_or_err()) ));
			let blk_data = ::kernel::lib::as_byte_slice(&blk_data[..]);
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
		//log_trace!("remain {} (bulk)", buf.len() - read_bytes);
		while buf.len() - read_bytes >= self.fs_block_size()
		{
			let remain_blocks = (buf.len() - read_bytes)/self.fs_block_size();
			let (blkid, count) = try!(blocks.next_extent_or_err( remain_blocks as u32 ));
			let byte_count = count as usize * self.fs_block_size();
			try!(self.inode.fs.read_blocks(blkid, &mut buf[read_bytes ..][.. byte_count]));
			read_bytes += byte_count;
		}

		// 5. Read the trailing partial block
		//log_trace!("remain {} (tail)", buf.len() - read_bytes);
		if buf.len() - read_bytes > 0
		{
			let blk_data = try!(self.inode.fs.get_block_uncached( try!(blocks.next_or_err()) ));
			let blk_data = ::kernel::lib::as_byte_slice(&blk_data[..]);
			buf[read_bytes..].clone_from_slice(&blk_data);
			read_bytes = buf.len();
		}

		// 6. Return number of bytes read (which may be smaller than the original buffer length)
		Ok( read_bytes )
	}

	fn truncate(&self, newsize: u64) -> vfs::node::Result<u64> {
		if newsize == 0
		{
			todo!("truncate - 0");
		}
		else if newsize == self.inode.i_size()
		{
			Ok( newsize )
		}
		else if newsize < self.inode.i_size()
		{
			todo!("truncate - shrink");
		}
		else
		{
			todo!("truncate - grow");
		}
	}
	fn clear(&self, ofs: u64, size: u64) -> vfs::node::Result<()> {
		if self.inode.fs.is_readonly()
		{
			Err( vfs::Error::ReadOnlyFilesystem )
		}
		else if ofs >= self.inode.i_size() || size > self.inode.i_size() || ofs + size > self.inode.i_size() {
			Err( vfs::Error::InvalidParameter )
		}
		else {
			// 1. Leading partial
			// 2. Inner
			// 3. Trailing partial
			todo!("clear");
		}
	}
	fn write(&self, ofs: u64, buf: &[u8]) -> vfs::Result<usize> {
		if self.inode.fs.is_readonly()
		{
			Err( vfs::Error::ReadOnlyFilesystem )
		}
		else if ofs == self.inode.i_size()
		{
			todo!("write - extend");
		}
		else if ofs > self.inode.i_size() || buf.len() as u64 > self.inode.i_size() || ofs + buf.len() as u64 > self.inode.i_size() {
			Err( vfs::Error::InvalidParameter )
		}
		else {
			// NOTE: In this section, we're free to read-modify-write blocks without fear, as the VFS itself handles
			//       the file "borrow checking". A file race is the userland's problem (if a SharedRW handle is used)
			let (blk_idx, blk_ofs) = ::kernel::lib::num::div_rem(ofs, self.fs_block_size() as u64);
			let mut blocks = self.inode.blocks_from(blk_idx as u32);
			let mut written = 0;
			// 1. Leading partial
			let blk_ofs = blk_ofs as usize;
			if blk_ofs > 0
			{
				todo!("write - mutate - leading partial {}", blk_ofs);
				//written += blk_ofs;
			}
			// 2. Inner
			while buf.len() - written >= self.fs_block_size()
			{
				let remain_blocks = (buf.len() - written)/self.fs_block_size();
				let (blkid, count) = try!(blocks.next_extent_or_err( remain_blocks as u32 ));
				let byte_count = count as usize * self.fs_block_size();
				try!(self.inode.fs.write_blocks(blkid, &buf[written ..][.. byte_count]));
				written += byte_count;
			}
			// 3. Trailing partial
			let trailing_bytes = buf.len() - written;
			if trailing_bytes > 0
			{
				todo!("write - mutate - trailing partial {}", trailing_bytes);
			}

			Ok( written )
		}
	}
}

