//
//
//
//! 
use instance::InstancePtr;
use kernel::vfs;

pub struct Inode
{
	fs: InstancePtr,
	inode_idx: u32,
	ondisk: ::ondisk::Inode,
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
			})
	}
}

impl Inode
{
	pub fn i_mode_fmt(&self) -> u16 {
		self.ondisk.i_mode & ::ondisk::S_IFMT
	}
}

impl Inode
{
	pub fn get_id(&self) -> vfs::node::InodeId {
		self.inode_idx as vfs::node::InodeId
	}
}

impl Inode
{
	pub fn get_block_addr(&self, block_idx: u32) -> u32
	{
		let u32_per_fs_block = (self.fs.fs_block_size / ::core::mem::size_of::<u32>()) as u32;

		let si_base = 12;
		let di_base = 12 + u32_per_fs_block ;
		let ti_base = 12 + u32_per_fs_block + u32_per_fs_block*u32_per_fs_block;

		if block_idx < si_base
		{
			// Direct block
			self.ondisk.i_block[block_idx as usize]
		}
		else if block_idx < di_base
		{
			// Single-indirect block
			let idx = block_idx - si_base;
			// TODO: Have locally a mutex-protected cached filesystem block (linked to a global cache manager)
			todo!("Support single-indirect idx={}", idx);
		}
		else if block_idx < ti_base
		{
			// Double-indirect block
			let idx = block_idx - di_base;
			let (blk, idx) = (idx / u32_per_fs_block, idx % u32_per_fs_block);
			todo!("Support double-indirect {},{}", blk, idx);
		}
		else
		{
			// Triple-indirect block
			let idx = block_idx - ti_base;
			let (blk, idx) = (idx / u32_per_fs_block, idx % u32_per_fs_block);
			let (blk_o, blk_i) = (blk / u32_per_fs_block, blk % u32_per_fs_block);
			todo!("Support triple-indirect {},{},{}", blk_o, blk_i, idx);
		}
	}
}

