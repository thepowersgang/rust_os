//
//
//
//! 
use instance::InstancePtr;
use kernel::vfs;
use core::sync::atomic::{AtomicBool,Ordering};

pub struct Inode
{
	pub fs: InstancePtr,
	inode_idx: u32,
	ondisk: ::ondisk::Inode,

	is_dirty: AtomicBool,
}

impl Inode
{
	pub fn from_id(fs: InstancePtr, id: u32) -> vfs::Result<Inode>
	{
		let od = try!( fs.read_inode(id) );
		Ok(Inode {
			fs: fs,
			inode_idx: id,
			ondisk: od,
			is_dirty: AtomicBool::new(false),
			})
	}

	pub fn dec_link_count(&self) {
		todo!("Inode::dec_link_count");
	}
	pub fn inc_link_count(&self) {
		todo!("Inode::inc_link_count");
	}


	pub fn flush(&self) -> vfs::Result<()>
	{
		if self.is_dirty.swap(false, Ordering::Relaxed)
		{
			try!(self.fs.write_inode(self.inode_idx, &self.ondisk));
		}
		Ok( () )
	}
}

impl Drop for Inode
{
	fn drop(&mut self)
	{
		if self.is_dirty.load(Ordering::Relaxed)
		{
			log_warning!("Inode::drop - Dirty node being dropped, writing back and ignoring errors");
			let _ = self.flush();
		}
	}
}

impl Inode
{
	pub fn i_mode_fmt(&self) -> u16 {
		self.ondisk.i_mode & ::ondisk::S_IFMT
	}
	pub fn i_size(&self) -> u64 {
		self.ondisk.i_size as u64
	}
}

impl Inode
{
	pub fn get_id(&self) -> vfs::node::InodeId {
		self.inode_idx as vfs::node::InodeId
	}
	pub fn max_blocks(&self) -> u32 {
		let n_blocks = (self.i_size() + self.fs.fs_block_size as u64 - 1) / self.fs.fs_block_size as u64;
		if n_blocks > ::core::u32::MAX as u64 {
			::core::u32::MAX
		}
		else {
			n_blocks as u32
		}
	}
}

impl Inode
{
	pub fn write_lock(&self) -> ::kernel::sync::rwlock::Write<()> {
		todo!("Inode::write_lock");
	}

	pub fn get_extent_from_block(&self, block_idx: u32, max_blocks: u32) -> vfs::node::Result<(u32, u32)>
	{
		let u32_per_fs_block = (self.fs.fs_block_size / ::core::mem::size_of::<u32>()) as u32;

		const SI_BLOCK: usize = 12;
		const DI_BLOCK: usize = 13;
		const TI_BLOCK: usize = 14;
		
		let si_base = SI_BLOCK as u32;
		let di_base = si_base + u32_per_fs_block;
		let ti_base = di_base + u32_per_fs_block*u32_per_fs_block;

		if block_idx < si_base
		{
			let fs_start = self.ondisk.i_block[block_idx as usize];
			let max_blocks = ::core::cmp::min( si_base - block_idx, max_blocks );
			for num in 1 .. max_blocks
			{
				if fs_start + num != self.ondisk.i_block[(block_idx + num) as usize] {
					return Ok( (fs_start, num) );
				}
			}
			Ok( (fs_start, max_blocks) )
		}
		else if block_idx < di_base
		{
			let idx = block_idx - si_base;
			// TODO: Have locally a mutex-protected cached filesystem block (linked to a global cache manager)
			let si_block = try!( self.fs.get_block( self.ondisk.i_block[SI_BLOCK] ) );
			
			let fs_start = si_block[idx as usize];
			let max_blocks = ::core::cmp::min( di_base - block_idx, max_blocks );
			for num in 1 .. max_blocks
			{
				if fs_start + num != si_block[ (idx + num) as usize ] {
					return Ok( (fs_start, num) );
				}
			}
			Ok( (fs_start, max_blocks) )
		}
		else if block_idx < ti_base
		{
			let idx = block_idx - di_base;
			let (blk, idx) = (idx / u32_per_fs_block, idx % u32_per_fs_block);
			let di_block = try!( self.fs.get_block( self.ondisk.i_block[DI_BLOCK] ) );
			let di_block = try!( self.fs.get_block( di_block[blk as usize] ) );


			let fs_start = di_block[idx as usize];
			let max_blocks = ::core::cmp::min( u32_per_fs_block - idx, max_blocks );
			for num in 1 .. max_blocks
			{
				if fs_start + num != di_block[ (idx + num) as usize ] {
					return Ok( (fs_start, num) );
				}
			}
			Ok( (fs_start, max_blocks) )
		}
		else
		{
			// Triple-indirect block
			let idx = block_idx - ti_base;
			let (blk, idx) = (idx / u32_per_fs_block, idx % u32_per_fs_block);
			let (blk_o, blk_i) = (blk / u32_per_fs_block, blk % u32_per_fs_block);
			let ti_block = try!( self.fs.get_block( self.ondisk.i_block[TI_BLOCK] ) );
			let ti_block = try!( self.fs.get_block( ti_block[blk_o as usize] ) );
			let ti_block = try!( self.fs.get_block( ti_block[blk_i as usize] ) );


			let fs_start = ti_block[idx as usize];
			let max_blocks = ::core::cmp::min( u32_per_fs_block - idx, max_blocks );
			for num in 1 .. max_blocks
			{
				if fs_start + num != ti_block[ (idx + num) as usize ] {
					return Ok( (fs_start, num) );
				}
			}
			Ok( (fs_start, max_blocks) )
		}
	}

	pub fn get_block_addr(&self, block_idx: u32) -> vfs::node::Result<u32>
	{
		let u32_per_fs_block = (self.fs.fs_block_size / ::core::mem::size_of::<u32>()) as u32;

		let si_base = 12;
		let di_base = 12 + u32_per_fs_block ;
		let ti_base = 12 + u32_per_fs_block + u32_per_fs_block*u32_per_fs_block;

		if block_idx < si_base
		{
			// Direct block
			Ok( self.ondisk.i_block[block_idx as usize] )
		}
		else if block_idx < di_base
		{
			// Single-indirect block
			let idx = block_idx - si_base;
			// TODO: Have locally a mutex-protected cached filesystem block (linked to a global cache manager)
			let si_block = try!( self.fs.get_block( self.ondisk.i_block[12] ) );
			Ok( si_block[ idx as usize ] )
		}
		else if block_idx < ti_base
		{
			// Double-indirect block
			let idx = block_idx - di_base;
			let (blk, idx) = (idx / u32_per_fs_block, idx % u32_per_fs_block);
			let di_block = try!( self.fs.get_block( self.ondisk.i_block[13] ) );
			let di_block = try!( self.fs.get_block( di_block[blk as usize] ) );
			Ok( di_block[idx as usize] )
		}
		else
		{
			// Triple-indirect block
			let idx = block_idx - ti_base;
			let (blk, idx) = (idx / u32_per_fs_block, idx % u32_per_fs_block);
			let (blk_o, blk_i) = (blk / u32_per_fs_block, blk % u32_per_fs_block);
			let ti_block = try!( self.fs.get_block( self.ondisk.i_block[14] ) );
			let ti_block = try!( self.fs.get_block( ti_block[blk_o as usize] ) );
			let ti_block = try!( self.fs.get_block( ti_block[blk_i as usize] ) );
			Ok( ti_block[idx as usize] )
		}
	}


	pub fn blocks(&self) -> Blocks//impl Iterator<Item=u32>
	{
		Blocks {
			inode: self,
			inner_idx: 0,
			}
		//(0 .. self.i_size() / self.fs.fs_block_size).map(|i| self.get_block_addr(i))
	}
	pub fn blocks_from(&self, start: u32) -> Blocks//impl Iterator<Item=u32>
	{
		Blocks {
			inode: self,
			inner_idx: start,
			}
	}
}

/// Iterator over block numbers owned by an inode
pub struct Blocks<'a>
{
	inode: &'a Inode,
	inner_idx: u32,
}
impl<'a> Blocks<'a>
{
	pub fn next_or_err(&mut self) -> ::kernel::vfs::Result<u32>
	{
		self.next().ok_or( ::kernel::vfs::Error::Unknown("Unexpected end of block list") )
	}

	pub fn next_extent_or_err(&mut self, max: u32) -> ::kernel::vfs::Result<(u32, u32)>
	{
		if self.inner_idx >= self.inode.max_blocks() {
			Err( ::kernel::vfs::Error::Unknown("Unexpected end of block list") )
		}
		else {
			let max = ::core::cmp::min(self.inode.max_blocks() - self.inner_idx, max);

			let rv = try!(self.inode.get_extent_from_block(self.inner_idx, max));
			self.inner_idx += rv.1;
			Ok(rv)
		}
	}
}
impl<'a> Iterator for Blocks<'a>
{
	type Item = u32;
	fn next(&mut self) -> Option<u32>
	{
		if self.inner_idx >= self.inode.max_blocks() {
			None
		}
		else {
			let ba = match self.inode.get_block_addr(self.inner_idx)
				{
				Ok(v) => v,
				Err(_) => return None,
				};
			self.inner_idx += 1;
			Some(ba)
		}
	}
}

