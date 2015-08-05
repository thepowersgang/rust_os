// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/vfs.rs
/// Virtual Filesystem interface
use kernel::prelude::*;

use kernel::memory::freeze::{Freeze,FreezeMut};
use super::{objects,ObjectHandle};
use super::values;
use super::Error;
use super::SyscallArg;

impl ::core::convert::From<::kernel::vfs::Error> for ::values::VFSError {
	fn from(v: ::kernel::vfs::Error) -> Self {
		use kernel::vfs::Error;
		use values::VFSError;
		match v
		{
		Error::NotFound     => VFSError::FileNotFound,
		Error::TypeMismatch => VFSError::TypeError,
		Error::PermissionDenied => VFSError::PermissionDenied,
		Error::Locked => VFSError::FileLocked,
		v @ _ => todo!("VFS Error {:?}", v),
		}
	}
}

pub fn openfile(path: &[u8], mode: u32) -> Result<ObjectHandle,u32> {
	
	let mode = match mode
		{
		1 => ::kernel::vfs::handle::FileOpenMode::SharedRO,
		2 => ::kernel::vfs::handle::FileOpenMode::Execute,
		_ => todo!("Unkown mode {:x}", mode),
		};
	match ::kernel::vfs::handle::File::open(::kernel::vfs::Path::new(path), mode)
	{
	Ok(h) => Ok( objects::new_object( File(h) ) ),
	Err(e) => todo!("syscall_vfs_openfile - e={:?}", e),
	}
}

struct File(::kernel::vfs::handle::File);
impl objects::Object for File
{
	const CLASS: u16 = values::CLASS_VFS_FILE;
	fn class(&self) -> u16 { Self::CLASS }
	fn as_any(&self) -> &Any { self }
	fn handle_syscall(&self, call: u16, mut args: &[usize]) -> Result<u64,Error> {
		match call
		{
		values::VFS_FILE_READAT => {
			let ofs = try!( <u64>::get_arg(&mut args) );
			let mut dest = try!( <FreezeMut<[u8]>>::get_arg(&mut args) );
			log_debug!("File::readat({}, {:p}+{} bytes)", ofs, dest.as_ptr(), dest.len());
			match self.0.read(ofs, &mut dest)
			{
			Ok(count) => Ok(count as u64),
			Err(e) => todo!("File::handle_syscall READAT Error {:?}", e),
			}
			},
		values::VFS_FILE_WRITEAT => {
			let ofs = try!( <u64>::get_arg(&mut args) );
			let src = try!( <Freeze<[u8]>>::get_arg(&mut args) );
			log_debug!("File::writeat({}, {:p}+{} bytes)", ofs, src.as_ptr(), src.len());
			match self.0.write(ofs, &src)
			{
			Ok(count) => Ok(count as u64),
			Err(e) => todo!("File::handle_syscall WRITEAT Error {:?}", e),
			}
			},
		values::VFS_FILE_MEMMAP => {
			let ofs = try!( <u64>::get_arg(&mut args) );
			let size = try!( <usize>::get_arg(&mut args) );
			let addr = try!( <usize>::get_arg(&mut args) );
			let mode = match try!( <u8>::get_arg(&mut args) )
				{
				0 => ::kernel::vfs::handle::MemoryMapMode::ReadOnly,
				1 => ::kernel::vfs::handle::MemoryMapMode::Execute,
				2 => ::kernel::vfs::handle::MemoryMapMode::COW,
				3 => ::kernel::vfs::handle::MemoryMapMode::WriteBack,
				v @ _ => {
					log_log!("VFS_FILE_MEMMAP - Bad protection mode {}", v);
					return Err( Error::BadValue );
					},
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
	fn bind_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
	fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
}



pub fn opendir(path: &[u8]) -> Result<ObjectHandle,u32>
{
	match ::kernel::vfs::handle::Dir::open(::kernel::vfs::Path::new(path))
	{
	Ok(h) => Ok( objects::new_object(Dir::new(h)) ),
	Err(e) => Err( Into::into( <::values::VFSError as From<_>>::from(e) ) ),
	}
}

struct Dir {
	inner: ::kernel::sync::Mutex<DirInner>,
}
impl Dir {
	fn new(handle: ::kernel::vfs::handle::Dir) -> Dir {
		Dir {
			inner: ::kernel::sync::Mutex::new( DirInner {
				handle: handle,
				lower_ofs: 0,
				cache: Default::default(),
				} )
		}
	}
}

impl objects::Object for Dir
{
	const CLASS: u16 = values::CLASS_VFS_DIR;
	fn class(&self) -> u16 { Self::CLASS }
	fn as_any(&self) -> &Any { self }
	fn handle_syscall(&self, call: u16, mut args: &[usize]) -> Result<u64,Error> {
		match call
		{
		values::VFS_DIR_READENT => {
			let mut name = try!(<FreezeMut<[u8]>>::get_arg(&mut args));
			Ok( super::from_result( match self.inner.lock().read_ent()
				{
				Err(e) => Err(e),
				Ok(None) => Ok(0),
				Ok(Some((_ino, s))) => {
					name.clone_from_slice( s.as_bytes() );
					Ok(s.len() as u32)
					},
				}) )
			},
		_ => todo!("Dir::handle_syscall({}, ...)", call),
		}
	}
	fn bind_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
	fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
}

type DirEnt = (::kernel::vfs::node::InodeId, ::kernel::lib::byte_str::ByteString);

struct DirInner {
	handle: ::kernel::vfs::handle::Dir,
	lower_ofs: usize,
	
	cache: DirEntCache,
}
impl DirInner
{
	fn read_ent(&mut self) -> Result< Option<DirEnt>, ::values::VFSError > {
		if let Some(e) = self.cache.next() {
			Ok( Some(e) )
		}
		else {
			let (ofs, count) = try!(self.handle.read_ents(self.lower_ofs, self.cache.ents()));
			self.lower_ofs = ofs;
			if count > 0 {
				self.cache.repopulate(count as u8);
				Ok( self.cache.next() )
			}
			else {
				Ok( None )
			}
		}
	}
}

#[derive(Debug)]
struct DirEntCache {
	count: u8,
	ofs: u8,
	ents: [DirEnt; 4],
}
impl Default for DirEntCache {
	fn default() -> DirEntCache {
		DirEntCache {
			count: 0,
			ofs: 0,
			ents: [Default::default(), Default::default(), Default::default(), Default::default()],
		}
	}
}
impl DirEntCache {
	fn ents(&mut self) -> &mut [DirEnt] {
		&mut self.ents
	}
	fn repopulate(&mut self, count: u8) {
		self.count = count;
		self.ofs = 0;
	}
	fn next(&mut self) -> Option<DirEnt> {
		log_debug!("DirEntCache self = {:?}", self);
		if self.ofs == self.count {
			None
		}
		else {
			self.ofs += 1;
			Some( ::core::mem::replace(&mut self.ents[self.ofs as usize - 1], (0, ::kernel::lib::byte_str::ByteString::new())) )
		}
	}
}
