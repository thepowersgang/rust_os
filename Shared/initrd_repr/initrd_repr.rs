#![no_std]

pub const MAGIC_NUMBER: u32 = 0x71FF0EF1;
	
#[repr(C)]
pub struct Header {
	pub magic: u32,
	pub total_length: u32,
	pub node_count: u32,
	pub root_length: u32,
}
#[repr(C)]
pub struct Inode {
	pub length: u32,
	pub ofs: u32,
	pub ty: u8,
	pub _reserved: [u8; 3]
	// TODO: Anything else?
}

pub const NODE_TY_REGULAR: u8 = 0;
pub const NODE_TY_DIRECTORY: u8 = 1;
//pub const NODE_TY_SYMLINK: u8 = 2;

#[repr(C)]
pub struct DirEntry {
	pub node: u32,
	pub filename: [u8; 64-4],
}