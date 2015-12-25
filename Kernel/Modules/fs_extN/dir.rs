// "Tifflin" Kernel - ext2/3/4 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_extN/dir.rs
//! Directory handling
use kernel::prelude::*;
use kernel::vfs;
use kernel::lib::byte_str::ByteStr;


pub struct Dir
{
	inode: ::inodes::Inode,
}


impl Dir
{
	pub fn new(inode: ::inodes::Inode) -> Dir
	{
		Dir {
			inode: inode,
			}
	}
}

impl vfs::node::NodeBase for Dir
{
	fn get_id(&self) -> vfs::node::InodeId {
		self.inode.get_id()
	}
}
impl vfs::node::Dir for Dir
{
	fn lookup(&self, name: &ByteStr) -> vfs::node::Result<vfs::node::InodeId>
	{
		// Linear search
		// TODO: Later revisions have B+ trees
		'outer: for blkid in self.inode.blocks()
		{
			let blk_data = try!(self.inode.fs.get_block(blkid));
			
			for ent in DirEnts(&blk_data)
			{
				if ent.d_rec_len == 0 {
					break 'outer;
				}
				if &ent.d_name == name.as_ref()
				{
					return Ok( ent.d_inode as vfs::node::InodeId );
				}
			}
		}
		Err(vfs::node::IoError::NotFound)
	}
	fn read(&self, start_ofs: usize, callback: &mut vfs::node::ReadDirCallback) -> vfs::node::Result<usize>
	{
		let (blk_idx, ofs) = ::kernel::lib::num::div_rem(start_ofs, self.inode.fs.fs_block_size);
		let mut blk_ofs = start_ofs - ofs;

		// 1. Seek to the requested block
		let mut blocks = self.inode.blocks_from(blk_idx as u32);

		// 2. Handle a partial block
		if ofs != 0
		{
			let blkid = match blocks.next()
				{
				Some(v) => v,
				None => return Ok(start_ofs),
				};
			let blk_data = try!(self.inode.fs.get_block(blkid));

			if let Some(ofs) = read_from_block(ofs, &blk_data, callback)
			{
				 return Ok(blk_ofs + ofs);
			}
			blk_ofs += self.inode.fs.fs_block_size;
		}
		// 3. Handle aligned blocks
		for blkid in blocks
		{
			let blk_data = try!(self.inode.fs.get_block(blkid));
			if let Some(ofs) = read_from_block(0, &blk_data, callback)
			{
				 return Ok(blk_ofs + ofs);
			}
			blk_ofs += self.inode.fs.fs_block_size;
		}
		
		return Ok( blk_ofs );

		// -----

		// Helper: Returns Some(blk_ofs) when a zero-length record is hit
		fn read_from_block(mut cur_ofs: usize, data: &[u32], callback: &mut vfs::node::ReadDirCallback) -> Option<usize>
		{
			for ent in DirEnts(&data[cur_ofs / 4..])
			{
				if ent.d_rec_len == 0 {
					return Some(cur_ofs);
				}

				if ent.d_name.len() > 0
				{
					callback(ent.d_inode as vfs::node::InodeId, &mut ent.d_name.iter().cloned());
				}
				
				cur_ofs += ent.u32_len();
			}

			None
		}
	}

	fn create(&self, name: &ByteStr, nodetype: vfs::node::NodeType) -> vfs::node::Result<vfs::node::InodeId> {
		todo!("create");
	}
	fn link(&self, name: &ByteStr, inode: vfs::node::InodeId) -> vfs::node::Result<()> {
		todo!("link");
	}
	fn unlink(&self, name: &ByteStr) -> vfs::node::Result<()> {
		todo!("unlink");
	}
}


struct DirEnts<'a>(&'a [u32]);

impl<'a> Iterator for DirEnts<'a>
{
	type Item = &'a ::ondisk::DirEnt;
	fn next(&mut self) -> Option<Self::Item>
	{
		if self.0.len() < ::ondisk::DIRENT_MIN_SIZE / 4 {
			// Consistency error: This shouldn't happen
			None
		}
		else {
			let rv = match ::ondisk::DirEnt::new(self.0)
				{
				Some(v) => v,
				None => {
					// Consistency Error: Record length too long
					return None;
					},
				};
			self.0 = &self.0[.. rv.u32_len() / 4];
			Some( rv )
		}
	}
}

