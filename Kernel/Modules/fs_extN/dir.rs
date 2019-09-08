// "Tifflin" Kernel - ext2/3/4 Filesystem Driver
// - By John Hodge (thePowersGang)
//
// Modules/fs_extN/dir.rs
//! Directory handling
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


	/// Returns (block_index, offset)
	fn find_name(&self, name: &ByteStr) -> vfs::node::Result<(usize, usize, vfs::node::InodeId)>
	{
		// Linear search
		// TODO: Later revisions have B+ trees
		for (blk_index, vol_blk) in self.inode.blocks().enumerate()
		{
			let blk_data = try!(self.inode.fs.get_block(vol_blk));
			
			let mut offset = 0;
			for ent in DirEnts(&blk_data)
			{
				if ent.d_rec_len == 0 {
					return Err( vfs::Error::InconsistentFilesystem );
				}
				else if ent.d_inode == 0 {
					return Err(vfs::Error::NotFound);
				}
				else if &ent.d_name == name.as_ref()
				{
					return Ok( (blk_index, offset, ent.d_inode as vfs::node::InodeId) );
				}
				else {
					offset += ent.u32_len() * 4;
				}
			}
		}
		Err(vfs::Error::NotFound)
	}


	fn find_free(&self, name: &ByteStr) -> vfs::node::Result<(u32, usize)>
	{
		assert!(name.len() <= 255);
		// Linear search
		// TODO: Later revisions have B+ trees
		for (blk_index, vol_blk) in self.inode.blocks().enumerate()
		{
			let blk_data = try!(self.inode.fs.get_block(vol_blk));
			
			let mut offset = 0;
			for ent in DirEnts(&blk_data)
			{
				if ent.d_rec_len == 0 {
					return Err( vfs::Error::InconsistentFilesystem );
				}
				else if ent.d_inode == 0 && (ent.d_rec_len - 8) <= name.len() as u16
				{
					// Free entry with sufficient space!
					return Ok( (blk_index as u32, offset) );
				}
				else {
					offset += ent.u32_len() * 4;
				}
			}
		}

		todo!("Dir::find_free - expand directory");
	}

	fn add_dir_ent(&self, name: &ByteStr, inode: u32) -> Result<(), vfs::Error>
	{
		// 1. Find a suitable slot
		let (blk, ofs) = try!(self.find_free(name));
		// 2. Fill said slot
		let vol_blk = try!( self.inode.blocks_from(blk as u32).next_or_err() );
		self.inode.fs.edit_block(vol_blk, |blk_data| {
				match ::ondisk::DirEnt::new_mut(&mut blk_data[ofs/4 ..])
				{
				None => return Err(vfs::Error::InconsistentFilesystem),
				Some(ent) => {
					ent.d_name_len = name.len() as u8;
					ent.d_inode = inode as u32;
					},
				}
				// - Now that name length is longer, update the name
				::ondisk::DirEnt::new_mut(&mut blk_data[ofs/4 ..]).unwrap()
					.d_name.clone_from_slice( name.as_ref() );
				Ok( () )
				})
	}
}

impl vfs::node::NodeBase for Dir
{
	fn get_id(&self) -> vfs::node::InodeId {
		self.inode.get_id()
	}
	fn get_any(&self) -> &dyn core::any::Any {
		self
	}
}
impl vfs::node::Dir for Dir
{
	fn lookup(&self, name: &ByteStr) -> vfs::Result<vfs::node::InodeId>
	{
		if name.len() == 0 {
			Err(vfs::Error::NotFound)
		}
		else {
			let (_, _, rv) = try!(self.find_name(name));
			Ok( rv )
		}
	}
	fn read(&self, start_ofs: usize, callback: &mut vfs::node::ReadDirCallback) -> vfs::Result<usize>
	{
		log_trace!("read(start_ofs={}, ...)", start_ofs);
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
				log_debug!("ent = {:?}", ent);
				if ent.d_rec_len == 0 {
					return Some(cur_ofs);
				}

				// Zero-length names should be ignored
				if ent.d_inode == 0 || ent.d_name.len() == 0 {
				}
				// Ignore . and .. entries
				else if &ent.d_name == b"." || &ent.d_name == b".." {
				}
				else {
					if callback(ent.d_inode as vfs::node::InodeId, &mut ent.d_name.iter().cloned()) == false {
						return Some(cur_ofs);
					}
				}
				
				cur_ofs += ent.u32_len() * 4;
			}

			None
		}
	}

	fn create(&self, name: &ByteStr, nodetype: vfs::node::NodeType) -> vfs::node::Result<vfs::node::InodeId> {
		if self.inode.fs.is_readonly()
		{
			Err( vfs::Error::ReadOnlyFilesystem )
		}
		else
		{
			let _lh = self.inode.write_lock();

			let ino_id = try!( self.inode.fs.allocate_inode(self.inode.get_id() as u32, nodetype) );
			match self.add_dir_ent(name, ino_id)
			{
			Ok(()) => Ok(ino_id as vfs::node::InodeId),
			Err(e) => {
				// Call with_inode to force the inode to be deallocated
				let _ = self.inode.fs.with_inode(ino_id, |_| Ok(()));
				Err(e)
				},
			}
		}
	}
	fn link(&self, name: &ByteStr, node: &dyn vfs::node::NodeBase) -> vfs::node::Result<()> {
		if self.inode.fs.is_readonly()
		{
			Err( vfs::Error::ReadOnlyFilesystem )
		}
		else if name == ""
		{
			Err(vfs::Error::InvalidParameter)
		}
		else if name.len() > 255
		{
			Err(vfs::Error::Unknown("Filename too long"))
		}
		else
		{
			let _lh = self.inode.write_lock();

			// TODO: How can I be sure that the passed inode number is valid? (or that it stays valid)
			let inode = node.get_id();
			try!(self.add_dir_ent(name, inode as u32));
			// 3. Update inode's link count
			try!(self.inode.fs.with_inode(inode as u32, |ino| {
				ino.inc_link_count();
				Ok( () )
				}));
			
			todo!("link(name={:?}, inode={:?})", name, inode);
		}
	}
	fn unlink(&self, name: &ByteStr) -> vfs::node::Result<()> {
		if self.inode.fs.is_readonly()
		{
			Err( vfs::Error::ReadOnlyFilesystem )
		}
		else if name == ""
		{
			Err( vfs::Error::InvalidParameter )
		}
		else
		{
			let _lh = self.inode.write_lock();

			let (blk, ofs, _) = try!(self.find_name(name));

			let vol_blk = try!( self.inode.blocks_from(blk as u32).next_or_err() );

			self.inode.fs.edit_block(vol_blk, |blk_data| {
				match ::ondisk::DirEnt::new_mut(&mut blk_data[ofs/4 ..])
				{
				None => return Err(vfs::Error::InconsistentFilesystem),
				Some(ent) => {
					// Clear name length
					ent.d_name_len = 0;
					// Decrement inode's reference count
					try!(self.inode.fs.with_inode(ent.d_inode, |ino| {
						ino.dec_link_count();
						Ok( () )
						}));
					Ok( () )
					},
				}
				})
		}
	}
}


struct DirEnts<'a>(&'a [u32]);

impl<'a> Iterator for DirEnts<'a>
{
	type Item = &'a ::ondisk::DirEnt;
	fn next(&mut self) -> Option<Self::Item>
	{
		if self.0.len() == 0 {
			// Complete
			None
		}
		else if self.0.len() < ::ondisk::DIRENT_MIN_SIZE / 4 {
			// Consistency error: This shouldn't happen
			log_warning!("Consistency error: Remaining len {} < min {}", self.0.len()*4, ::ondisk::DIRENT_MIN_SIZE);
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
			self.0 = &self.0[rv.u32_len() .. ];
			Some( rv )
		}
	}
}

