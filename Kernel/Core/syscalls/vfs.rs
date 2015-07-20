// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/vfs.rs
/// Virtual Filesystem interface
use prelude::*;

use super::{objects,ObjectHandle};
use super::values;
use super::Error;
use super::SyscallArg;

pub fn openfile(path: &[u8], mode: u32) -> Result<ObjectHandle,u32> {
	struct File(::vfs::handle::File);

	impl objects::Object for File {
		fn handle_syscall(&self, call: u16, mut args: &[usize]) -> Result<u64,Error> {
			match call
			{
			values::VFS_FILE_READAT => {
				let ofs = try!( <u64>::get_arg(&mut args) );
				let dest = try!( <&mut [u8]>::get_arg(&mut args) );
				log_debug!("File::readat({}, {:p}+{} bytes)", ofs, dest.as_ptr(), dest.len());
				match self.0.read(ofs, dest)
				{
				Ok(count) => Ok(count as u64),
				Err(e) => todo!("File::handle_syscall READAT Error {:?}", e),
				}
				},
			values::VFS_FILE_WRITEAT => {
				todo!("File::handle_syscall WRITEAT");
				},
			values::VFS_FILE_MEMMAP => {
				let ofs = try!( <u64>::get_arg(&mut args) );
				let size = try!( <usize>::get_arg(&mut args) );
				let addr = try!( <usize>::get_arg(&mut args) );
				let mode = match try!( <u8>::get_arg(&mut args) )
					{
					0 => ::vfs::handle::MemoryMapMode::ReadOnly,
					1 => ::vfs::handle::MemoryMapMode::Execute,
					2 => ::vfs::handle::MemoryMapMode::COW,
					3 => ::vfs::handle::MemoryMapMode::WriteBack,
					_ => return Err( Error::BadValue ),
					};
				log_debug!("VFS_FILE_MEMMAP({:#x}, {:#x}+{}, {:?}", ofs, addr, size, mode);
				
				match self.0.memory_map(addr, ofs, size, mode)
				{
				Ok(h) => {
					log_warning!("TODO: register memory map handle with object table");
					::core::mem::forget(h);
					Ok(0)
					},
				Err(e) => todo!("File::handle_syscall MEMMAP Error {:?}", e),
				}
				},
			_ => todo!("File::handle_syscall({}, ...)", call),
			}
		}
        fn bind_wait(&self, flags: u32, obj: &mut ::threads::SleepObject) -> u32 { 0 }
        fn clear_wait(&self, flags: u32, obj: &mut ::threads::SleepObject) -> u32 { 0 }
	}
	
	let mode = match mode
		{
		1 => ::vfs::handle::FileOpenMode::SharedRO,
		2 => ::vfs::handle::FileOpenMode::Execute,
		_ => todo!("Unkown mode {:x}", mode),
		};
	match ::vfs::handle::File::open(::vfs::Path::new(path), mode)
	{
	Ok(h) => Ok( objects::new_object( File(h) ) ),
	Err(e) => todo!("syscall_vfs_openfile - e={:?}", e),
	}
}
