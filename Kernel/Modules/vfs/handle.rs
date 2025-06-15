// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/handle.rs
//! Opened file interface (top level VFS interface)
#[allow(unused_imports)]
use ::kernel::prelude::*;
use ::kernel::lib::byte_str::{ByteStr,ByteString};
use ::kernel::PAGE_SIZE;
use super::node::{NodeType};
use super::node_cache::{CacheHandle};
use super::Path;

#[derive(Debug,Clone)]
/// Open without caring what the file type is (e.g. enumeration)
pub struct Any {
	node: CacheHandle,
}
#[derive(Debug,Clone)]
/// Normal file
pub struct File {
	node: super::node_cache::CacheHandleFile,
	mode: FileOpenMode,
}
#[derive(Debug,Clone)]
/// Directory (for enumeration)
pub struct Dir {
	node: super::node_cache::CacheHandleDir,
}
#[derive(Debug,Clone)]
/// Symbolic link (allows reading the link contents)
pub struct Symlink {
	node: CacheHandle,
}
#[derive(Debug,Clone)]
/// Special file (?API exposed)
pub struct Special {
	_node: CacheHandle,
}

#[derive(Debug,Clone)]
pub enum FileOpenMode
{
	/// No access to file data (can just read the size)
	NoDataAccess,
	/// Shared read-only, multiple readers but no writers visible
	///
	/// When opened in this manner, the file contents cannot change, but it might extend
	SharedRO,
	/// Open for execution (multiple readers)
	///
	/// No file changes visible to handles, must be an executable file
	Execute,
	/// Eclusive read-write, denies any other opens while held (TODO: except Append?)
	///
	/// No changes to the file will be visible to the user (as the file is locked)
	ExclRW,
	/// Unique read-write, does Copy-on-write to create a new file
	///
	/// No changes to the file will be visible to any other process, and will be discarded on close
	UniqueRW,
	/// Append only (allows multiple readers/writers)
	///
	/// Cannot read, all writes go to the end of the file (a write call is atomic)
	Append,
	/// Unsynchronised read-write
	///
	/// No synchronisation at all, fails if any other open type is active.
	Unsynch,
}

#[derive(Debug)]
pub enum MemoryMapMode
{
	/// Read-only mapping of a file
	ReadOnly,
	/// Executable mapping of a file
	Execute,
	/// Copy-on-write (used for executable files)
	COW,
	/// Allows writing to the backing file
	WriteBack,
}

impl Any
{
	/// Open the specified path (not caring what the actual type is)
	pub fn open(path: &Path) -> super::Result<Any> {
		log_trace!("Any::open({:?})", path);
		let node = CacheHandle::from_path(path)?;
		Ok(Any { node: node })
	}

	/// Get the node class of the handle
	pub fn get_class(&self) -> super::node_cache::NodeClass {
		self.node.get_class()
	}
	
	/// Upgrade the handle to a directory handle
	pub fn into_dir(self) -> super::Result<Dir> {
		Ok(Dir { node: self.node.into_dir()? })
	}

	/// Upgrade the handle to a file handle
	pub fn into_file(self, mode: FileOpenMode) -> super::Result<File> {
		Ok(File::from_node(self.node.into_file()?, mode)?)
	}
	
	/// Upgrade the handle to a symlink handle
	pub fn into_symlink(self) -> super::Result<Symlink> {
		if self.node.is_symlink() {
			Ok(Symlink { node: self.node })
		}
		else {
			Err(super::Error::TypeMismatch)
		}
	}
}

pub struct MemoryMapHandle<'a>
{
	handle: &'a File,
	base: *mut (),
	len: usize,
}

impl File
{
	/// Open the specified path as a file
	pub fn open(path: &Path, mode: FileOpenMode) -> super::Result<File> {
		let node = CacheHandle::from_path(path)?.into_file()?;
		Self::from_node(node, mode)
	}

	fn from_node(node: super::node_cache::CacheHandleFile, mode: FileOpenMode) -> super::Result<File> {
		match mode
		{
		FileOpenMode::NoDataAccess => {},
		// TODO: Check permissions (must be readable in current context)
		FileOpenMode::SharedRO => { node.file_lock_shared()?; },
		// TODO: Check permissions (must be writable in current context)
		FileOpenMode::Append => { node.file_lock_shared()?; },
		// TODO: Check permissions (must be executable in current context)
		FileOpenMode::Execute => { node.file_lock_shared()?; },
		// TODO: Check permissions (must be writable in current context)
		FileOpenMode::ExclRW => { node.file_lock_exclusive()?; }
		FileOpenMode::Unsynch => { node.file_lock_unsynch()?; }
		FileOpenMode::UniqueRW => todo!("UniqueRW - CoW"),
		}
		Ok(File { node: node, mode: mode })
	}
	
	pub fn size(&self) -> u64 {
		self.node.get_valid_size()
	}
	pub fn truncate(&self) {
		todo!("Handle::truncate()");
	}

	/// Read data from the file at the specified offset
	///
	/// Returns the number of read bytes (which might be less than the size of the input
	/// slice).
	pub fn read(&self, ofs: u64, dst: &mut [u8]) -> super::Result<usize> {
		match self.mode
		{
		FileOpenMode::NoDataAccess => return Err(super::Error::PermissionDenied),
		_ => self.node.read(ofs, dst),
		}
	}
	/// Write data to the file (offset is ignored if open for appending)
	pub fn write(&self, ofs: u64, src: &[u8]) -> super::Result<usize> {
		match self.mode
		{
		FileOpenMode::NoDataAccess => return Err(super::Error::PermissionDenied),
		FileOpenMode::SharedRO => return Err(super::Error::PermissionDenied),
		FileOpenMode::Execute => return Err(super::Error::PermissionDenied),
		FileOpenMode::Append => self.node.append(src),
		FileOpenMode::ExclRW
		|FileOpenMode::UniqueRW
		|FileOpenMode::Unsynch => self.node.write(ofs, src),
		}
	}

	
	/// Map a file into the address space
	pub fn memory_map(&self, address: usize, ofs: u64, size: usize, mode: MemoryMapMode) -> super::Result<MemoryMapHandle<'_>> {
		log_debug!("memory_map(self={{mode:{:?}}}, address={:#x}, ofs={:#x}, size={:#x}, mode={:?})",
			self.mode, address, ofs, size, mode);
		// - Check that this file is opened in a sufficient mode to allow this form of mapping
		match mode
		{
		// Read only - Pretty much anything
		MemoryMapMode::ReadOnly => match self.mode
			{
			FileOpenMode::Execute
			|FileOpenMode::SharedRO => {},
			//FileOpenMode::ExclRW => /* NOTE: Needs extra checks to ensure that aliasing does not occur */
			//FileOpenMode::UniqueRW => /* NOTE: Needs extra checks to ensure that aliasing does not occur */
			_ => return Err(super::Error::PermissionDenied),
			},
		// Executable - Execute mode only
		MemoryMapMode::Execute => match self.mode
			{
			FileOpenMode::Execute => {},
			_ => return Err(super::Error::PermissionDenied),
			},
		// COW - Execute mode only
		// - TODO: Could allow COW of readonly files? (as soon as it's written, the page is detached from the file)
		MemoryMapMode::COW => match self.mode
			{
			FileOpenMode::Execute => {},
			//FileOpenMode::SharedRO => {},
			_ => return Err(super::Error::PermissionDenied),
			},
		// Writeback - Requires exclusive access to the file (or a copy)
		MemoryMapMode::WriteBack => match self.mode
			{
			//FileOpenMode::ExclRW => /* NOTE: Needs extra checks to ensure that aliasing does not occur */
			//FileOpenMode::UniqueRW => /* NOTE: Needs extra checks to ensure that aliasing does not occur */
			_ => return Err(super::Error::PermissionDenied),
			},
		}
		
		// TODO: Handle unaligned mappings somehow
		// - Depends on several qirks:
		//  > Unaligned address could write to an existing page (converting it to a private) - But how would that interact with existing mappings?
		//  > Unaligned sizes would usually cause a new anon mapping, but if its unaligned becuase of EOF, it should just be COW as usual
		assert!(address % PAGE_SIZE == 0, "TODO: Unaligned memory_map (address={})", address);
		assert!(size % PAGE_SIZE == 0, "TODO: Unaligned memory_map (size={})", size);
		if address % PAGE_SIZE != (ofs % PAGE_SIZE as u64) as usize {
			return Err( super::Error::Unknown("memory_map alignment mismatch") );
		}
		// - Limit checking (ofs + size must be within size of the file)
		// TODO: Limit checking
		// - Reserve the region to be mapped (reserve sticks a zero page in)
		let page_count = size / PAGE_SIZE;
		let mut resv = match ::kernel::memory::virt::reserve(address as *mut (), page_count)
			{
			Ok(v) => v,
			Err(e) => {
				log_notice!("mmap reserve error {:?}", e);
				return Err( super::Error::Locked );
				},
			};
		// - Obtain handles to each cached page, and map into the reservation
		for i in 0 .. page_count {
			let page = ofs / PAGE_SIZE as u64 + i as u64;
			// 1. Search the node for this particular page
			//let lh = self.page_cache.read();
			//  - If found, map over region
			// 2. Drop lock, read data from file, and try again
			//drop(lh)
			self.node.read(page * PAGE_SIZE as u64, resv.get_mut_page(i))?;
			// 3. Acquire write on lock, and attempt to insert a handle to this page
			//let lh = self.page_cache.write();
			//match lh.try_insert(pag, self.get_page_handle(i))
			//{
			//Ok(_) => {},	// Inserted correctly
			//Err(h) => {	// Another handle made a page for this first
			//	resv.map_at(i, h);	// - Map over our original attempt
			//	},
			//}
		}
		resv.finalise( match mode
			{
			MemoryMapMode::ReadOnly  => ::kernel::memory::virt::ProtectionMode::UserRO,
			MemoryMapMode::Execute   => ::kernel::memory::virt::ProtectionMode::UserRX,
			MemoryMapMode::COW       => ::kernel::memory::virt::ProtectionMode::UserCOW,
			MemoryMapMode::WriteBack => ::kernel::memory::virt::ProtectionMode::UserRW,
			})
			.unwrap();
		log_debug!("- Mapped at {:p} + {:#x}", address as *mut (), page_count * PAGE_SIZE);
		Ok(MemoryMapHandle {
			handle: self,
			base: address as *mut (),
			len: page_count * PAGE_SIZE,
			})
	}
}
impl ::core::ops::Drop for File
{
	fn drop(&mut self) {
		match self.mode
		{
		FileOpenMode::NoDataAccess => {},
		FileOpenMode::SharedRO
		| FileOpenMode::Append
		| FileOpenMode::Execute => self.node.file_unlock_shared(),
		FileOpenMode::ExclRW => self.node.file_unlock_exclusive(),
		FileOpenMode::Unsynch => self.node.file_unlock_unsync(),
		FileOpenMode::UniqueRW => todo!("File::drop() - mode={:?}", self.mode),
		}
	}
}

impl<'a> Drop for MemoryMapHandle<'a>
{
	fn drop(&mut self)
	{
		let _ = self.handle;
		assert_eq!(self.len % PAGE_SIZE, 0, "TODO: Handle unaligned lengths in MemoryMapHandle::drop");
		assert_eq!(self.base as usize % PAGE_SIZE, 0, "TODO: Handle unaligned addresses in MemoryMapHandle::drop");
		let npages = self.len / PAGE_SIZE;
		// SAFE: This is a uniquely owned handle
		unsafe {
			::kernel::memory::virt::unmap(self.base, npages);
		}
	}
}

impl Dir
{
	/// Open a provided path as a directory
	pub fn open(path: &Path) -> super::Result<Dir> {
		Any::open(path)?.into_dir()
	}
	
	/// Iterate names within the directory
	pub fn iter(&self) -> DirIter<'_> {
		DirIter {
			handle: self,
			ents: [
				Default::default(), Default::default(),
				Default::default(), Default::default(),
				],
			pos: 0,
			ofs: 0,
			count: 0,
		}
	}
	
	/// Create a new directory
	pub fn mkdir(&self, name: impl AsRef<ByteStr>) -> super::Result<Dir> {
		let node = self.node.create(name.as_ref(), NodeType::Dir)?;
		Ok( Dir { node: node.into_dir()? } )
	}
	/// Create a new symbolic link
	pub fn symlink(&self, name: impl AsRef<ByteStr>, target: &Path) -> super::Result<()> {
		self.node.create(name.as_ref(), NodeType::Symlink(target))?;
		Ok( () )
	}
	/// Create a new file (opened exclusively)
	pub fn create_file(&self, name: impl AsRef<ByteStr>) -> super::Result<File> {
		let node = self.node.create(name.as_ref(), NodeType::File)?;
		File::from_node(node.into_file()?, FileOpenMode::ExclRW)
	}

	/// Open a child of this node
	pub fn open_child(&self, name: &ByteStr) -> super::Result<Any> {
		let node = self.node.open_child(name)?;
		Ok(Any { node: node })
	}

	pub fn open_child_path(&self, path: &Path) -> super::Result<Any> {
		let node = CacheHandle::from_path_at_node(self.node.clone(), path)?;
		Ok(Any{ node: node })
	}


	/// RETURN: next position
	pub fn read_ents(&self, pos: usize, ents: &mut super::node::ReadDirCallback) -> super::Result<usize> {
		self.node.read_dir(pos, ents)
	}
}

pub struct DirIter<'a> {
	handle: &'a Dir,
	count: usize,
	ofs: usize,
	pos: usize,
	ents: [(super::node::InodeId,ByteString); 4],
}
impl<'a> ::core::iter::Iterator for DirIter<'a> {
	type Item = ByteString;
	fn next(&mut self) -> Option<ByteString> {
		if self.ofs == self.count
		{
			self.count = 0;
			match self.handle.node.read_dir(self.pos, &mut |inode, name| {
				self.ents[self.count].0 = inode;
				self.ents[self.count].1 = name.collect();
				self.count += 1;
				self.count < 4
				})
			{
			Err(e) => {
				log_warning!("Error while iterating dir: {:?}", e);
				return None;
				},
			Ok(next) => {
				self.pos = next;
				},
			}
			if self.count == 0 {
				return None;
			}
			self.ofs = 0;
		}
		
		let ent_str = &mut self.ents[self.ofs].1;
		self.ofs += 1;
		Some( ::core::mem::replace(ent_str, ByteString::new()) )
	}
}


impl Symlink
{
	pub fn open(path: &Path) -> super::Result<Symlink> {
		Any::open(path)?.into_symlink()
	}
	pub fn get_target(&self) -> super::Result<ByteString> {
		self.node.get_target()
	}
}

