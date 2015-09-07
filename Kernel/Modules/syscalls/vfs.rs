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
use kernel::vfs::{handle,node};
use kernel::vfs::Path;


macro_rules! map_enums {
	( ($a:ident, $b:ident) match ($v:expr) { $( ($l:ident $($extra:tt)*), )* } ) => {
		match $v
		{
		$( $a::$l => map_enums!(@arm $b $l : $($extra)*) ),*
		}
		};
	(@arm $b:ident $l:ident : => @$r:ident) => { ($b::$r)  };
	(@arm $b:ident $l:ident : => $v:expr) => { $v };
	(@arm $b:ident $l:ident : ) => { ($b::$l) };
}

impl_from! {
	From<::kernel::vfs::Error>(v) for ::values::VFSError {{
		use kernel::vfs::Error;
		use values::VFSError;
		match v
		{
		Error::NotFound     => VFSError::FileNotFound,
		Error::TypeMismatch => VFSError::TypeError,
		Error::PermissionDenied => VFSError::PermissionDenied,
		Error::Locked => VFSError::FileLocked,
		Error::MalformedPath => VFSError::MalformedPath,
		Error::NonDirComponent
		| Error::RecursionDepthExceeded
		| Error::BlockIoError(_)
			=> todo!("VFS Error - {:?}", v),
		Error::Unknown(reason) => todo!("VFS Error Unknown - '{}'", reason),
		}
	}}
	From<node::NodeClass>(v) for ::values::VFSNodeType {
		match v
		{
		node::NodeClass::File => ::values::VFSNodeType::File,
		node::NodeClass::Dir => ::values::VFSNodeType::Dir,
		node::NodeClass::Symlink => ::values::VFSNodeType::Symlink,
		node::NodeClass::Special => ::values::VFSNodeType::Special,
		}
	}

	From<::values::VFSFileOpenMode>(v) for handle::FileOpenMode {{
		use values::VFSFileOpenMode;
		use kernel::vfs::handle::FileOpenMode;
		map_enums!(
			(VFSFileOpenMode, FileOpenMode)
			match (v) {
				(ReadOnly => @SharedRO),
				(Execute),
				(ExclRW),
				(UniqueRW),
				(Append),
				(Unsynch),
			}
		)
		//match v
		//{
		//::values::VFSFileOpenMode::None => todo!(""),
		//::values::VFSFileOpenMode::ReadOnly => handle::FileOpenMode::SharedRO,
		//::values::VFSFileOpenMode::Execute => handle::FileOpenMode::Execute,
		//::values::VFSFileOpenMode::ExclRW => handle::FileOpenMode::ExclRW,
		//::values::VFSFileOpenMode::UniqueRW => handle::FileOpenMode::UniqueRW,
		//::values::VFSFileOpenMode::Append => handle::FileOpenMode::Append,
		//::values::VFSFileOpenMode::Unsynch => handle::FileOpenMode::Unsynch,
		//}
	}}
}

/// Convert a VFS result into an encoded syscall result
fn to_result<T>(r: Result<T, ::kernel::vfs::Error>) -> Result<T, u32> {
	r.map_err( |e| Into::into( <::values::VFSError as From<_>>::from(e) ) )
}


// --------------------------------------------------------------------
//
// --------------------------------------------------------------------

/// Open a bare file
pub fn opennode(path: &[u8]) -> Result<ObjectHandle,u32> {
	to_result( handle::Any::open( Path::new(path) ) )
		.map( |h| objects::new_object(Node(h)) )
}
struct Node( handle::Any );
impl objects::Object for Node
{
	const CLASS: u16 = values::CLASS_VFS_NODE;
	fn class(&self) -> u16 { Self::CLASS }
	fn as_any(&self) -> &Any { self }
	fn handle_syscall(&self, call: u16, mut args: &[usize]) -> Result<u64,Error> {
		match call
		{
		values::VFS_NODE_GETTYPE => {
			let v32: u32 = ::values::VFSNodeType::from( self.0.get_class() ).into();
			Ok( v32 as u64 )
			},
		values::VFS_NODE_TOFILE => {
			let mode = try!( <u8>::get_arg(&mut args) );

			let mode = ::values::VFSFileOpenMode::from(mode);
			todo!("VFS_NODE_TOFILE({:?}", mode)
			},
		values::VFS_NODE_TODIR => todo!("VFS_NODE_TODIR"),
		values::VFS_NODE_TOLINK => {
			// TODO: I'd like this to reuse this object handle etc... but that's not possible with
			// the current structure.
			// - Has to clone the Any to avoid moving out of self (cheap operation, just a refcount inc).
			let objres = to_result(self.0.clone().to_symlink())
				.map( |h| objects::new_object(Link(h)) );
			Ok(super::from_result( objres ))
			},
		_ => todo!("Node::handle_syscall({}, ...)", call),
		}
	}
	fn bind_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
	fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
}


// --------------------------------------------------------------------
//
// --------------------------------------------------------------------

pub fn openfile(path: &[u8], mode: u8) -> Result<ObjectHandle,u32> {
	
	log_trace!("openfile({:?}, mode={:?})", path, mode);
	let mode: handle::FileOpenMode = ::values::VFSFileOpenMode::from(mode).into();
	let path = Path::new(path);
	log_trace!("- openfile({:?}, mode={:?})", path, mode);
	to_result( handle::File::open(path, mode) )
		.map( |h| objects::new_object(File(h)) )
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
		values::VFS_FILE_GETSIZE => {
			Ok( self.0.size() )
			},
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


// --------------------------------------------------------------------
//
// --------------------------------------------------------------------

pub fn opendir(path: &[u8]) -> Result<ObjectHandle,u32>
{
	to_result( handle::Dir::open(::kernel::vfs::Path::new(path)) )
		.map(|h| objects::new_object(Dir::new(h)) )
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


// --------------------------------------------------------------------
//
// --------------------------------------------------------------------

pub fn openlink(path: &[u8]) -> Result<ObjectHandle,u32>
{
	to_result( handle::Symlink::open(::kernel::vfs::Path::new(path)) )
		.map(|h| objects::new_object(Link(h)) )
}

struct Link(handle::Symlink);
impl objects::Object for Link
{
	const CLASS: u16 = values::CLASS_VFS_LINK;
	fn class(&self) -> u16 { Self::CLASS }
	fn as_any(&self) -> &Any { self }
	fn handle_syscall(&self, call: u16, mut args: &[usize]) -> Result<u64,Error> {
		match call
		{
		values::VFS_LINK_READ => {
			let mut buf = try!(<FreezeMut<[u8]>>::get_arg(&mut args));
			let res = to_result( self.0.get_target() )
				.map(|tgt| {
					buf.clone_from_slice(tgt.as_bytes());
					tgt.len() as u32
					});
			Ok( super::from_result(res) )
			},
		_ => todo!("Node::handle_syscall({}, ...)", call),
		}
	}
	fn bind_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
	fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
}

