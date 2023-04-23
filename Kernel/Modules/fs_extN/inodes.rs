//
//
//
//! 
use instance::{InstancePtr,InstanceInner};
use core::sync::atomic::{AtomicBool,Ordering};

pub struct Inode
{
	pub fs: InstancePtr,
	inode_idx: u32,

	is_dirty: AtomicBool,
	/// Inode data, with a lock
	on_disk: ::kernel::sync::RwLock<crate::ondisk::Inode>,
	/// Lock used for directories to maintain internal consistency
	dir_lock: ::kernel::sync::Mutex<()>,
}
impl Inode
{
	/// Load an inode from the disk
	pub fn from_id(fs: InstancePtr, id: u32) -> ::vfs::Result<Inode>
	{
		let od = try!( fs.read_inode(id) );
		Ok(Inode {
			fs: fs,
			inode_idx: id,
			is_dirty: AtomicBool::new(false),
			on_disk: ::kernel::sync::RwLock::new(od),
			dir_lock: Default::default(),
			})
	}
}
impl Drop for Inode
{
	fn drop(&mut self)
	{
		if self.is_dirty.load(Ordering::Relaxed) {
			log_warning!("Inode::drop - Dirty node being dropped, writing back and ignoring errors");
			let _ = self.flush();
		}
	}
}

impl Inode
{
	/// Commit all changes to the disk
	pub fn flush(&self) -> vfs::Result<()>
	{
		if self.is_dirty.swap(false, Ordering::Relaxed)
		{
			try!(self.fs.write_inode(self.inode_idx, &self.on_disk.read()));
		}
		Ok( () )
	}

	pub fn dec_link_count(&self) {
		todo!("Inode::dec_link_count");
	}
	pub fn inc_link_count(&self) {
		todo!("Inode::inc_link_count");
	}

	/// Obtain the inode ID
	pub fn get_id(&self) -> vfs::node::InodeId {
		self.inode_idx as vfs::node::InodeId
	}
	/// Obtain the node contents consistency lock (used for directories)
	pub fn lock_dir(&self) -> ::kernel::sync::mutex::HeldMutex<'_, ()> {
		self.dir_lock.lock()
	}
}
impl Inode
{
	pub fn lock_read(&self) -> InodeHandleRead<'_> {
		InodeHandleRead {
			parent: self,
			lock: self.on_disk.read(),
		}
	}
	pub fn lock_write(&self) -> InodeHandleWrite<'_> {
		InodeHandleWrite {
			parent: self,
			lock: self.on_disk.write(),
		}
	}
}
pub struct InodeHandleRead<'a> {
	parent: &'a Inode,
	lock: ::kernel::sync::rwlock::Read<'a, ::ondisk::Inode>,
}
pub struct InodeHandleWrite<'a> {
	parent: &'a Inode,
	lock: ::kernel::sync::rwlock::Write<'a, ::ondisk::Inode>,
}
macro_rules! common_methods {
	($($(#[$attr:meta])* pub fn $name:ident(&$self:ident$(, $a:ident : $t:ty)*) -> $rv:ty $b:block)+) => {
		pub trait InodeHandleTrait<'a> {
			fn fs(&self) -> &'a crate::instance::InstanceInner;
			$(
			$(#[$attr])*
			fn $name(&$self$(, $a : $t)*) -> $rv;
			)+
		}
		impl<'a> InodeHandleTrait<'a> for InodeHandleRead<'a> {
			fn fs(&self) -> &'a crate::instance::InstanceInner {
				&self.parent.fs
			}
			$(
			$(#[$attr])*
			fn $name(&$self$(, $a : $t)*) -> $rv $b
			)+
		}
		impl<'a> InodeHandleRead<'a> {
			$(
			#[allow(dead_code)]
			pub fn $name(&$self$(, $a : $t)*) -> $rv {
				InodeHandleTrait::$name($self $(, $a)*)
			}
			)+
		}
		impl<'a> InodeHandleTrait<'a> for InodeHandleWrite<'a> {
			fn fs(&self) -> &'a crate::instance::InstanceInner {
				&self.parent.fs
			}
			$(
			fn $name(&$self$(, $a : $t)*) -> $rv $b
			)+
		}
		impl<'a> InodeHandleWrite<'a> {
			$(
			#[allow(dead_code)]
			$(#[$attr])*
			pub fn $name(&$self$(, $a : $t)*) -> $rv {
				InodeHandleTrait::$name($self $(, $a)*)
			}
			)+
		}
	}
}
common_methods! {
	pub fn i_mode_fmt(&self) -> u16 {
		self.lock.i_mode_fmt()
	}
	pub fn i_size(&self) -> u64 {
		self.lock.i_size(&self.parent.fs)
	}
	pub fn get_extent_from_block(&self, block_idx: u32, max_blocks: u32) -> vfs::node::Result<(u32, u32)> {
		self.lock.get_extent_from_block(&self.parent.fs, block_idx, max_blocks)
	}
	pub fn get_block_addr(&self, block_idx: u32) -> vfs::node::Result<u32> {
		self.lock.get_block_addr(&self.parent.fs, block_idx)
	}
	/// The maximum number of blocks needed to contain the file size
	pub fn max_blocks(&self) -> u32 {
		self.lock.max_blocks(&self.parent.fs)
	}
	pub fn blocks(&self) -> Blocks {
		Blocks {
			fs: &self.parent.fs,
			ondisk: &self.lock,
			inner_idx: 0,
			}
	}
	pub fn blocks_from(&self, start: u32) -> Blocks {
		Blocks {
			fs: &self.parent.fs,
			ondisk: &self.lock,
			inner_idx: start,
			}
	}
}
impl<'a> InodeHandleWrite<'a> {
	pub fn set_i_size(&mut self, new_size: u64) -> vfs::node::Result<()> {
		self.lock.set_i_size(&self.parent.fs, new_size)
	}
}

impl crate::ondisk::Inode
{
	fn i_mode_fmt(&self) -> u16 {
		self.i_mode & ::ondisk::S_IFMT
	}
	fn i_size(&self, fs: &InstanceInner) -> u64 {
		self.i_size as u64 | (if fs.has_feature_ro_compat(crate::ondisk::FEAT_RO_COMPAT_LARGE_FILE) { (self.i_dir_acl as u64) << 32 } else { 0 })
	}
	fn set_i_size(&mut self, fs: &InstanceInner, s: u64) -> vfs::node::Result<()> {
		if fs.has_feature_ro_compat(crate::ondisk::FEAT_RO_COMPAT_LARGE_FILE) {
			self.i_dir_acl = (s >> 32) as u32;
		}
		else {
			if s > u32::MAX as u64 {
				self.i_size = u32::MAX;
				return Err(vfs::Error::InvalidParameter);
			}
		}
		self.i_size = s as u32;
		Ok( () )
	}

	fn max_blocks(&self, fs: &InstanceInner) -> u32 {
		let n_blocks = (self.i_size(fs) + fs.fs_block_size as u64 - 1) / fs.fs_block_size as u64;
		if n_blocks > ::core::u32::MAX as u64 {
			::core::u32::MAX
		}
		else {
			n_blocks as u32
		}
	}
	fn get_extent_from_block(&self, fs: &InstanceInner, block_idx: u32, max_blocks: u32) -> vfs::node::Result<(u32, u32)>
	{
		let u32_per_fs_block = (fs.fs_block_size / ::core::mem::size_of::<u32>()) as u32;

		const SI_BLOCK: usize = 12;
		const DI_BLOCK: usize = 13;
		const TI_BLOCK: usize = 14;
		
		let si_base = SI_BLOCK as u32;
		let di_base = si_base + u32_per_fs_block;
		let ti_base = di_base + u32_per_fs_block*u32_per_fs_block;

		if block_idx < si_base
		{
			let fs_start = self.i_block[block_idx as usize];
			let max_blocks = ::core::cmp::min( si_base - block_idx, max_blocks );
			for num in 1 .. max_blocks
			{
				if fs_start + num != self.i_block[(block_idx + num) as usize] {
					return Ok( (fs_start, num) );
				}
			}
			Ok( (fs_start, max_blocks) )
		}
		else if block_idx < di_base
		{
			let idx = block_idx - si_base;
			// TODO: Have locally a mutex-protected cached filesystem block (linked to a global cache manager)
			let si_block = try!( fs.get_block( self.i_block[SI_BLOCK] ) );
			
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
			let di_block = try!( fs.get_block( self.i_block[DI_BLOCK] ) );
			let di_block = try!( fs.get_block( di_block[blk as usize] ) );


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
			let ti_block = try!( fs.get_block( self.i_block[TI_BLOCK] ) );
			let ti_block = try!( fs.get_block( ti_block[blk_o as usize] ) );
			let ti_block = try!( fs.get_block( ti_block[blk_i as usize] ) );


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

	pub fn get_block_addr(&self, fs: &InstanceInner, block_idx: u32) -> vfs::node::Result<u32>
	{
		let u32_per_fs_block = (fs.fs_block_size / ::core::mem::size_of::<u32>()) as u32;

		let si_base = 12;
		let di_base = 12 + u32_per_fs_block ;
		let ti_base = 12 + u32_per_fs_block + u32_per_fs_block*u32_per_fs_block;

		if block_idx < si_base
		{
			// Direct block
			Ok( self.i_block[block_idx as usize] )
		}
		else if block_idx < di_base
		{
			// Single-indirect block
			let idx = block_idx - si_base;
			// TODO: Have locally a mutex-protected cached filesystem block (linked to a global cache manager)
			let si_block = try!( fs.get_block( self.i_block[12] ) );
			Ok( si_block[ idx as usize ] )
		}
		else if block_idx < ti_base
		{
			// Double-indirect block
			let idx = block_idx - di_base;
			let (blk, idx) = (idx / u32_per_fs_block, idx % u32_per_fs_block);
			let di_block = try!( fs.get_block( self.i_block[13] ) );
			let di_block = try!( fs.get_block( di_block[blk as usize] ) );
			Ok( di_block[idx as usize] )
		}
		else
		{
			// Triple-indirect block
			let idx = block_idx - ti_base;
			let (blk, idx) = (idx / u32_per_fs_block, idx % u32_per_fs_block);
			let (blk_o, blk_i) = (blk / u32_per_fs_block, blk % u32_per_fs_block);
			let ti_block = try!( fs.get_block( self.i_block[14] ) );
			let ti_block = try!( fs.get_block( ti_block[blk_o as usize] ) );
			let ti_block = try!( fs.get_block( ti_block[blk_i as usize] ) );
			Ok( ti_block[idx as usize] )
		}
	}
}

/// Iterator over block numbers owned by an inode
pub struct Blocks<'a>
{
	fs: &'a InstanceInner,
	ondisk: &'a crate::ondisk::Inode,
	inner_idx: u32,
}
impl<'a> Blocks<'a>
{
	pub fn next_or_err(&mut self) -> ::vfs::Result<u32> {
		self.next().ok_or( ::vfs::Error::Unknown("Unexpected end of block list") )
	}

	pub fn next_extent_or_err(&mut self, max: u32) -> ::vfs::Result<(u32, u32)> {
		let max_blocks = self.ondisk.max_blocks(self.fs);
		if self.inner_idx >= max_blocks {
			Err( ::vfs::Error::Unknown("Unexpected end of block list") )
		}
		else {
			let max = ::core::cmp::min(max_blocks - self.inner_idx, max);

			let rv = try!(self.ondisk.get_extent_from_block(self.fs, self.inner_idx, max));
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
		let max_blocks = self.ondisk.max_blocks(self.fs);
		if self.inner_idx >= max_blocks {
			None
		}
		else {
			let ba = match self.ondisk.get_block_addr(self.fs, self.inner_idx)
				{
				Ok(v) => v,
				Err(_) => return None,
				};
			self.inner_idx += 1;
			Some(ba)
		}
	}
}

