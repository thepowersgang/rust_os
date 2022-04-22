// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/syscalls/vfs.rs
/// Virtual Filesystem interface
use kernel::prelude::*;

use kernel::memory::freeze::{Freeze,FreezeMut};
use crate::objects;
use crate::values;
use crate::Error;
use crate::args::Args;
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
	From<::kernel::vfs::Error>(v) for values::VFSError {{
		use ::kernel::vfs::Error;
		use crate::values::VFSError;
		match v
		{
		Error::NotFound     => VFSError::FileNotFound,
		Error::TypeMismatch => VFSError::TypeError,
		Error::PermissionDenied => VFSError::PermissionDenied,
		Error::Locked => VFSError::FileLocked,
		Error::MalformedPath => VFSError::MalformedPath,
		Error::Unknown(reason) => todo!("VFS Error Unknown - '{}'", reason),
		_ => todo!("VFS Error - {:?}", v),
		}
	}}
	From<node::NodeClass>(v) for values::VFSNodeType {
		match v
		{
		node::NodeClass::File => values::VFSNodeType::File,
		node::NodeClass::Dir => values::VFSNodeType::Dir,
		node::NodeClass::Symlink => values::VFSNodeType::Symlink,
		node::NodeClass::Special => values::VFSNodeType::Special,
		}
	}

	From<values::VFSFileOpenMode>(v) for handle::FileOpenMode {{
		use crate::values::VFSFileOpenMode;
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
	}}
}

/// Convert a VFS result into an encoded syscall result
fn to_result<T>(r: Result<T, ::kernel::vfs::Error>) -> Result<T, u32> {
	r.map_err( |e| Into::into( <values::VFSError as From<_>>::from(e) ) )
}

pub fn init_handles(init_handle: ::kernel::vfs::handle::File) {
	// #1: Read-only root
	objects::push_as_unclaimed("ro:/", objects::new_object(Dir::new( {
		let root = handle::Dir::open(Path::new("/")).unwrap();
		//root.set_permissions( handle::Perms::readonly() );
		root
		})) );
	// #2: Initial file handle
	objects::new_object( File(init_handle) );

	// - Read-write handle to /
	objects::push_as_unclaimed("RwRoot", objects::new_object( Dir::new( handle::Dir::open(Path::new("/")).unwrap() ) ) );
}


// --------------------------------------------------------------------
//
// --------------------------------------------------------------------

struct Node( handle::Any );
impl objects::Object for Node
{
	fn class(&self) -> u16 { values::CLASS_VFS_NODE }
	fn as_any(&self) -> &dyn Any { self }
	fn try_clone(&self) -> Option<u32> {
		Some( objects::new_object( Node(self.0.clone()) ) )
	}
	fn handle_syscall_ref(&self, call: u16, _args: &mut Args) -> Result<u64,Error> {
		match call
		{
		values::VFS_NODE_GETTYPE => {
			log_debug!("VFS_NODE_GETTYPE()");
			let v32: u32 = values::VFSNodeType::from( self.0.get_class() ).into();
			Ok( v32 as u64 )
			},
		_ => objects::object_has_no_such_method_ref("vfs::Node", call),
		}
	}
	fn handle_syscall_val(&mut self, call: u16, args: &mut Args) -> Result<u64,Error> {
		// SAFE: Raw pointer coerced from &mut, caller forgets us
		let this = unsafe { ::core::ptr::read(self) };
		let inner = this.0;
		match call
		{
		values::VFS_NODE_TOFILE => {
			let mode: u8 = args.get()?;

			let mode = match crate::values::VFSFileOpenMode::try_from(mode)
				{
				Ok(v) => v,
				Err(_) => return Err( Error::BadValue ),
				};
			log_debug!("VFS_NODE_TOFILE({:?})", mode);

			let objres = to_result(inner.to_file(mode.into()))
				.map( |h| objects::new_object(File(h)) );
			Ok( super::from_result(objres) )
			},
		values::VFS_NODE_TODIR => {
			let objres = to_result(inner.to_dir())
				.map( |h| objects::new_object(Dir::new(h)) );
			Ok( super::from_result(objres) )
			},
		values::VFS_NODE_TOLINK => {
			let objres = to_result(inner.to_symlink())
				.map( |h| objects::new_object(Link(h)) );
			Ok(super::from_result( objres ))
			},
		_ => crate::objects::object_has_no_such_method_val("vfs::Node", call),
		}
	}
	fn bind_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
	fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
}


// --------------------------------------------------------------------
//
// --------------------------------------------------------------------

struct File(::kernel::vfs::handle::File);
impl objects::Object for File
{
	fn class(&self) -> u16 { values::CLASS_VFS_FILE }
	fn as_any(&self) -> &dyn Any { self }
	fn try_clone(&self) -> Option<u32> {
		Some( crate::objects::new_object( File(self.0.clone()) ) )
	}
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64,Error> {
		match call
		{
		values::VFS_FILE_GETSIZE => {
			Ok( self.0.size() )
			},
		values::VFS_FILE_READAT => {
			let ofs: u64 = args.get()?;
			let mut dest: FreezeMut<[u8]> = args.get()?;
			log_debug!("File::readat({}, {:p}+{} bytes)", ofs, dest.as_ptr(), dest.len());
			match self.0.read(ofs, &mut dest)
			{
			Ok(count) => Ok(count as u64),
			Err(e) => todo!("File::handle_syscall READAT Error {:?}", e),
			}
			},
		values::VFS_FILE_WRITEAT => {
			let ofs: u64 = args.get()?;
			let src: Freeze<[u8]> = args.get()?;
			log_debug!("File::writeat({}, {:p}+{} bytes)", ofs, src.as_ptr(), src.len());
			match self.0.write(ofs, &src)
			{
			Ok(count) => Ok(count as u64),
			Err(e) => todo!("File::handle_syscall WRITEAT Error {:?}", e),
			}
			},
		values::VFS_FILE_MEMMAP => {
			let ofs: u64 = args.get()?;
			let size: usize = args.get()?;
			let addr: usize = args.get()?;
			let mode = match args.get::<u8>()?
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
			log_debug!("VFS_FILE_MEMMAP({:#x}, {:#x}+{}, {:?})", ofs, addr, size, mode);
			
			match self.0.memory_map(addr, ofs, size, mode)
			{
			Ok(h) => {
				// TODO: I would like the map handle to be avaliable, but I'd like the user to be able to "forget" it
				// (so it becomes an indelible part of the address space).
				// - That would likely need a new system call similar to Drop
				// - XXX: The handle here has borrow of the file handle, so can't be stored as-is
				::core::mem::forget(h);
				Ok(0)
				},
			Err(e) => todo!("File::handle_syscall MEMMAP Error {:?}", e),
			}
			},
		_ => crate::objects::object_has_no_such_method_ref("vfs::File", call),
		}
	}
	fn bind_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
	fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
}

#[cfg(feature="native")]
/// Used by the native "kernel" to get a file object for `new_process`
pub fn get_file_handle(obj: u32) -> Result<::kernel::vfs::handle::File, crate::Error> {
	crate::objects::take_object::<crate::vfs::File>(obj)
		.map(|f| f.0)
}


// --------------------------------------------------------------------
//
// --------------------------------------------------------------------

struct Dir {
	handle: ::kernel::vfs::handle::Dir,
}
impl Dir {
	fn new(handle: ::kernel::vfs::handle::Dir) -> Dir {
		Dir {
			handle: handle,
		}
	}
}

impl objects::Object for Dir
{
	fn class(&self) -> u16 { values::CLASS_VFS_DIR }
	fn as_any(&self) -> &dyn Any { self }
	fn try_clone(&self) -> Option<u32> {
		Some( crate::objects::new_object( Dir { handle: self.handle.clone() } ) )
	}
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64,Error> {
		Ok(match call
		{
		values::VFS_DIR_OPENCHILD => {
			let name: Freeze<[u8]> = args.get()?;

			let name = ::kernel::lib::byte_str::ByteStr::new(&*name);
			log_debug!("VFS_DIR_OPENCHILD({:?})", name);

			super::from_result(
				to_result( self.handle.open_child(name) )
					.map( |h| objects::new_object(Node(h)) )
				)
			},
		values::VFS_DIR_OPENPATH => {
			let path: Freeze<[u8]> = args.get()?;

			let path = Path::new(&path);
			log_debug!("VFS_DIR_OPENPATH({:?})", path);
			super::from_result(
				to_result( self.handle.open_child_path(path) )
					.map( |h| objects::new_object(Node(h)) )
				)
			},
		values::VFS_DIR_ENUMERATE => {
			objects::new_object( DirIter::new( self.handle.clone() ) ) as u64
			},
		_ => return crate::objects::object_has_no_such_method_ref("vfs::Dir", call),
		})
	}
	//fn handle_syscall_val(self, call: u16, _args: &mut Args) -> Result<u64,Error> {
	//	::objects::object_has_no_such_method_val("vfs::Dir", call)
	//}
	fn bind_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
	fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
}

struct DirIter {
	handle: ::kernel::vfs::handle::Dir,
	inner: ::kernel::sync::Mutex<DirInner>,
}
impl DirIter {
	fn new(handle: ::kernel::vfs::handle::Dir) -> DirIter {
		DirIter {
			handle: handle,
			inner: ::kernel::sync::Mutex::new( DirInner {
				lower_ofs: 0,
				cache: Default::default(),
				} )
		}
	}
}
impl objects::Object for DirIter
{
	fn class(&self) -> u16 { values::CLASS_VFS_DIRITER }
	fn as_any(&self) -> &dyn Any { self }
	fn try_clone(&self) -> Option<u32> {
		None
	}
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64,Error> {
		Ok(match call
		{
		values::VFS_DIRITER_READENT => {
			let mut name: FreezeMut<[u8]> = args.get()?;
			log_debug!("VFS_DIRITER_READENT({:p}+{})", name.as_ptr(), name.len());

			super::from_result( match self.inner.lock().read_ent(&self.handle)
				{
				Err(e) => Err(e),
				Ok(None) => Ok(0),
				Ok(Some((_ino, s))) => {
					name[.. s.len()].clone_from_slice( s.as_bytes() );
					Ok(s.len() as u32)
					},
				})
			},
		_ => return crate::objects::object_has_no_such_method_ref("vfs::Dir", call),
		})
	}
	//fn handle_syscall_val(self, call: u16, _args: &mut Args) -> Result<u64,Error> {
	//	::objects::object_has_no_such_method_val("vfs::Dir", call)
	//}
	fn bind_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
	fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
}

type DirEnt = (::kernel::vfs::node::InodeId, ::kernel::lib::byte_str::ByteString);

struct DirInner {
	lower_ofs: usize,
	
	cache: DirEntCache,
}
impl DirInner
{
	fn read_ent(&mut self, handle: &::kernel::vfs::handle::Dir) -> Result< Option<DirEnt>, crate::values::VFSError >
	{
		if let Some(e) = self.cache.next() {
			Ok( Some(e) )
		}
		else {
			let cache = &mut self.cache;
			cache.reset();
			self.lower_ofs = handle.read_ents(self.lower_ofs, &mut |inode, name| {
				cache.push( (inode, name.collect()) );
				! cache.is_full()
				})?;
			Ok( cache.next() )
		}
	}
}

#[derive(Debug,Default)]
struct DirEntCache {
	count: u8,
	ofs: u8,
	ents: [DirEnt; 4],
}
impl DirEntCache {
	fn is_full(&self) -> bool {
		self.count == 4
	}
	fn push(&mut self, v: DirEnt) {
		assert!(self.count < 4);
		self.ents[self.count as usize] = v;
		self.count += 1;
	}
	fn reset(&mut self) {
		self.count = 0;
		self.ofs = 0;
	}
	fn next(&mut self) -> Option<DirEnt> {
		//log_trace!("DirEntCache::next() self = {:?}", self);
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

struct Link(handle::Symlink);
impl objects::Object for Link
{
	fn class(&self) -> u16 { values::CLASS_VFS_LINK }
	fn as_any(&self) -> &dyn Any { self }
	fn try_clone(&self) -> Option<u32> {
		Some( crate::objects::new_object( Link(self.0.clone()) ) )
	}
	fn handle_syscall_ref(&self, call: u16, args: &mut Args) -> Result<u64,Error> {
		match call
		{
		values::VFS_LINK_READ => {
			let mut buf: FreezeMut<[u8]> = args.get()?;
			log_debug!("VFS_LINK_READ({:p}+{})", buf.as_ptr(), buf.len());

			let res = to_result( self.0.get_target() )
				.map(|tgt| {
					buf.clone_from_slice(tgt.as_bytes());
					tgt.len() as u32
					});
			Ok( super::from_result(res) )
			},
		_ => crate::objects::object_has_no_such_method_ref("vfs::Link", call),
		}
	}
	//fn handle_syscall_val(self, call: u16, _args: &mut Args) -> Result<u64,Error> {
	//	::objects::object_has_no_such_method_val("vfs::Link", call)
	//}
	fn bind_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
	fn clear_wait(&self, _flags: u32, _obj: &mut ::kernel::threads::SleepObject) -> u32 { 0 }
}



// -
// -
//impl objects::Object for ::kernel::vfs::handle::MemoryMapHandle<'a>
//{
//}


