// "Tifflin" Kernel - ext2/3/4 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_extN/file.rs
//! Regular file
use ::vfs;

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
		self.inode.lock_read().i_size()
	}
	fn read(&self, ofs: u64, buf: &mut [u8]) -> vfs::node::Result<usize>
	{
		let inode = self.inode.lock_read();
		if ofs > inode.i_size() {
			return Err(vfs::Error::InvalidParameter);
		}
		if ofs == inode.i_size() {
			return Ok(0);
		}
		// 1. Restrict buffer size to avaiable bytes
		let avail_bytes = inode.i_size() - ofs;
		let buf = if buf.len() as u64 > avail_bytes {
				&mut buf[.. avail_bytes as usize]
			}
			else {
				buf
			};

		iter_blocks_range(&inode, ofs, buf.len(), &mut |block_range, data_range| {
			match block_range
			{
			BlockRef::Sub(blkid, sub_range) => {
				let blk_data = try!(self.inode.fs.get_block_uncached(blkid));
				buf[data_range].copy_from_slice(&blk_data[sub_range]);
				},
			BlockRef::Range(blkid, count) => {
				self.inode.fs.read_blocks(blkid, &mut buf[data_range])?;
				},
			}
			Ok( () )
			})
	}

	fn truncate(&self, new_size: u64) -> vfs::node::Result<u64> {
		let mut inode = self.inode.lock_write();
		let old_size = inode.i_size();
		if new_size == 0
		{
			todo!("truncate - 0");
		}
		else if new_size == old_size
		{
			Ok( new_size )
		}
		else if new_size < old_size
		{
			todo!("truncate - shrink");
		}
		else {
			ensure_blocks_present(&self.inode.fs, &mut inode, new_size)?;
			inode.set_i_size(new_size)?;
			// TODO: Risky cast? If truncating to a very large size
			iter_blocks_range(&inode, old_size, (new_size - old_size) as usize, &mut |_block_range, _data_range| {
				// TODO: Zero the blocks?
				Ok( () )
				})?;
			Ok( new_size )
		}
	}
	fn clear(&self, ofs: u64, size: u64) -> vfs::node::Result<()> {
		let inode = self.inode.lock_read();
		let i_size = inode.i_size();
		if self.inode.fs.is_readonly()
		{
			Err( vfs::Error::ReadOnlyFilesystem )
		}
		else if ofs >= i_size || size > i_size || ofs + size > i_size {
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
		let inode = self.inode.lock_read();
		let size = inode.i_size();
		if self.inode.fs.is_readonly()
		{
			Err( vfs::Error::ReadOnlyFilesystem )
		}
		else if ofs == size
		{
			drop(inode);
			let mut inode = self.inode.lock_write();
			if ofs == inode.i_size() {
			}
			else {
				// Race! Should this be possible? (shouldn't the vfs layer protect that?)
			}
			let new_size = ofs + buf.len() as u64;
			// Ensure that there are blocks allocated
			ensure_blocks_present(&self.inode.fs, &mut inode, new_size)?;
			// Extend the size
			inode.set_i_size(new_size)?;
			// Write data
			let rv = write_inner(&inode, ofs, buf)?;
			inode.set_i_size(ofs + rv as u64)?;
			Ok(rv)
		}
		else if ofs > size || buf.len() as u64 > size || ofs + buf.len() as u64 > size {
			Err( vfs::Error::InvalidParameter )
		}
		else {
			// NOTE: In this section, we're free to read-modify-write blocks without fear, as the VFS itself handles
			//       the file "borrow checking". A file race is the userland's problem (if a SharedRW handle is used)
			write_inner(&inode, ofs, buf)
		}
	}
}

fn write_inner(inode: &dyn super::inodes::InodeHandleTrait, ofs: u64, buf: &[u8]) -> vfs::Result<usize> {
	iter_blocks_range(inode, ofs, buf.len(), &mut |block_range, data_range| {
		match block_range
		{
		BlockRef::Sub(blkid, sub_range) => {
			inode.fs().edit_block(blkid, |data| {
				data[sub_range].copy_from_slice(&buf[data_range]);
				Ok( () )
				})?;
			},
		BlockRef::Range(blkid, count) => {
			inode.fs().write_blocks(blkid, &buf[data_range])?;
			},
		}
		Ok( () )
		})
}

enum BlockRef {
	Sub(u32, ::core::ops::Range<usize>),
	Range(u32, u32),
}
fn iter_blocks_range(inode: &dyn super::inodes::InodeHandleTrait, ofs: u64, len: usize, cb: &mut dyn FnMut(BlockRef, ::core::ops::Range<usize>)->vfs::Result<()>) -> vfs::Result<usize>
{
	let fs_block_size = inode.fs().fs_block_size;
	let (blk_idx, blk_ofs) = ::kernel::lib::num::div_rem(ofs, fs_block_size as u64);
	let mut blocks = inode.blocks_from(blk_idx as u32);
	let mut written = 0;
	// 1. Leading partial
	let blk_ofs = blk_ofs as usize;
	if blk_ofs > 0
	{
		let b = blocks.next_or_err()?;
		let len = usize::min(fs_block_size - blk_ofs, len);
		log_trace!("iter_blocks_range: Prefix B{} {}+{}", b, blk_ofs, len);
		cb( BlockRef::Sub(b, blk_ofs..blk_ofs+len), 0..len )?;
		written += len;
	}
	// 2. Inner
	while len - written >= fs_block_size
	{
		let remain_blocks = (len - written) / fs_block_size;
		let (blkid, count) = try!(blocks.next_extent_or_err( remain_blocks as u32 ));
		let byte_count = count as usize * fs_block_size;
		log_trace!("iter_blocks_range: Inner B{}+{} ({})", blkid, count, byte_count);
		cb( BlockRef::Range(blkid, count), written .. written + byte_count)?;
		written += byte_count;
	}
	// 3. Trailing partial
	let trailing_bytes = len - written;
	if trailing_bytes > 0
	{
		let b = blocks.next_or_err()?;
		log_trace!("iter_blocks_range: Suffix B{} 0+{}", b, trailing_bytes);
		cb( BlockRef::Sub(b, 0..trailing_bytes), written..written+len )?;
		written += trailing_bytes;
	}
	Ok( written )
}

fn ensure_blocks_present(fs: &super::instance::InstanceInner, lh: &mut super::inodes::InodeHandleWrite, size: u64) -> vfs::Result<()> {
	let nblocks = ::kernel::lib::num::div_up(size, fs.fs_block_size as u64);

	lh.ensure_blocks_allocated(0, nblocks as u32)
}

