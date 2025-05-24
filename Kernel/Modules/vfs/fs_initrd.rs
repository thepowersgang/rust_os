//! Read-only initial RAM Disk
//! 
//! A rather crude format, and very hacky code to run it
use core::convert::TryInto;
use kernel::lib::mem::Box;
use kernel::metadevs::storage::{IoError,VolumeHandle};
use core::mem::size_of;

extern crate initrd_repr as repr;

const BLOCK_SIZE: usize = 0x1000;

pub(crate) fn init() {
	::core::mem::forget(super::mount::DriverRegistration::new("initrd", &InitrdDriver));
}

fn trim_nuls(name: &[u8]) -> &[u8] {
	let l = name.iter().position(|v| *v == 0).unwrap_or(name.len());
	&name[..l]
}

/// The underlying "storage device" for an initrd
/// 
/// Acts like a normal device, except that if you read from block -1 it returns a magic number
/// and the slice (pointer and len) to the raw data.
pub struct InitrdVol {
	name: [u8; 6+1],
	handle: ::kernel::memory::virt::AllocHandle,
	ofs: u32,
	len: u32,
}
impl InitrdVol {
	/// UNSAFE: Takes raw physical memory locations, and thus can read from any memory
	pub unsafe fn new(base: u64, length: usize) -> Result<Self,()> {
		let handle = match ::kernel::memory::virt::map_hw_ro(
			base, (length + ::kernel::PAGE_SIZE-1) / ::kernel::PAGE_SIZE,
			"initrd"
			)
			{
			Ok(v) => v,
			Err(_e) => return Err(()),
			};
		let ofs = (base % (::kernel::PAGE_SIZE as u64)) as usize;

		static INDEX: ::core::sync::atomic::AtomicU32 = ::core::sync::atomic::AtomicU32::new(0);
		let index = INDEX.fetch_add(1, ::core::sync::atomic::Ordering::Relaxed);
		if index > 9 {
			return Err(());
		}

		let mut name = *b"initrd0";
		name[6] = b'0' + index as u8;
		Ok(Self {
			name,
			handle,
			ofs: ofs as u32,
			len: length as u32
		})
	}
}
impl ::kernel::metadevs::storage::PhysicalVolume for InitrdVol {
	fn name(&self) -> &str {
		::core::str::from_utf8(&self.name).unwrap()
	}

	fn blocksize(&self) -> usize {
		BLOCK_SIZE
	}

	fn capacity(&self) -> Option<u64> {
		Some(self.len as u64 / BLOCK_SIZE as u64)
	}

	fn read<'a>(&'a self, _: u8, blockidx: u64, count: usize, dst: &'a mut [u8]) -> kernel::metadevs::storage::AsyncIoResult<'a, usize> {
		// Handle the magic protocol
		if blockidx == !0 && count == 1 {
			dst[ 0..][..8].copy_from_slice(&(repr::MAGIC_NUMBER as u64).to_le_bytes() );
			dst[ 8..][..8].copy_from_slice(&(self.handle.as_slice::<u8>(self.ofs as usize, 0).as_ptr() as usize as u64).to_le_bytes());
			dst[16..][..8].copy_from_slice(&(self.len as u64).to_le_bytes());
			return Box::pin(async move { Ok(1) });
		}
		if blockidx as usize > (self.len as usize) / BLOCK_SIZE {
			return Box::pin(async move { Err(IoError::BadBlock) });
		}
		let count = count.min( self.len as usize / BLOCK_SIZE - blockidx as usize );
		assert!(dst.len() >= count * BLOCK_SIZE);
		let ofs = self.ofs as usize + blockidx as usize * BLOCK_SIZE;
		let src = self.handle.as_slice(ofs, dst.len());
		dst.copy_from_slice(src);
		
		Box::pin(async move { Ok(count) })
	}

	fn write<'a>(&'a self, _: u8, _: u64, _: usize, _: &'a [u8]) -> kernel::metadevs::storage::AsyncIoResult<'a, usize> {
		Box::pin(async move { Err(IoError::ReadOnly) })
	}

	fn wipe<'a>(&'a self, _: u64, _: usize) -> kernel::metadevs::storage::AsyncIoResult<'a,()> {
		Box::pin(async move { Err(IoError::ReadOnly) })
	}
}

pub struct InitrdDriver;
impl super::mount::Driver for InitrdDriver {
	fn detect(&self, vol: &VolumeHandle) -> crate::Result<usize> {
		let mut tmp = [0; 4096];
		::kernel::futures::block_on(vol.read_blocks(!0, &mut tmp))?;
		if tmp[..4] == repr::MAGIC_NUMBER.to_le_bytes() {
			Ok(3)
		}
		else {
			Ok(0)
		}
	}

	fn mount(&self, vol: VolumeHandle, _self_handle: crate::mount::SelfHandle) -> crate::Result<Box<dyn crate::mount::Filesystem>> {
		let mut tmp = [0; 4096];
		::kernel::futures::block_on(vol.read_blocks(!0, &mut tmp))?;
		if tmp[..4] != repr::MAGIC_NUMBER.to_le_bytes() {
			return Err(crate::Error::InconsistentFilesystem)
		}
		let ptr = u64::from_le_bytes(tmp[8..][..8].try_into().unwrap());
		let len = u64::from_le_bytes(tmp[16..][..8].try_into().unwrap());
		// SAFE: Since the magic passed, assume that the encoded slice is also valid
		let data = unsafe { ::core::slice::from_raw_parts(ptr as usize as *const u8, len as usize) };
		if data.as_ptr() as usize % ::core::mem::align_of::<repr::Header>() != 0 {
			return Err(crate::Error::InconsistentFilesystem)
		}
		if data.len() < size_of::<repr::Header>() {
			return Err(crate::Error::InconsistentFilesystem)
		}
		
		let instance = Inner {
			_vol_handle: vol,
			//self_handle,
			data
		};
		if data.len() < size_of::<repr::Header>() + instance.header().node_count as usize * size_of::<repr::Inode>() {
			return Err(crate::Error::InconsistentFilesystem)
		}
		if false {
			dump_file(data, instance.inodes(), "ROOT", 0, 0);
		}

		// SAFE: The ArefInner is going right in a box, and won't move until the box is dropped
		Ok(Box::new(InitrdInstance(unsafe { ::kernel::lib::mem::aref::ArefInner::new(instance) })))
	}
}
struct Inner {
	_vol_handle: VolumeHandle,
	//self_handle: super::mount::SelfHandle,
	data: &'static [u8],
}
impl Inner {
	fn header(&self) -> &repr::Header {
		// SAFE: Data alignment cheked by the original mount, and is valid for this type
		unsafe { &*(self.data.as_ptr() as *const repr::Header) }
	}
	fn inodes(&self) -> &[repr::Inode] {
		// SAFE: Data alignment cheked by the original mount, and is valid for this type
		unsafe {
			::core::slice::from_raw_parts(
				self.data.as_ptr().offset(size_of::<repr::Header>() as isize) as *const repr::Inode,
				self.header().node_count as usize
				)
		}
	}
	fn get_data(&self, ofs: u32, len: u32) -> crate::Result<&[u8]> {
		if ofs as usize > self.data.len() {
			return Err(crate::Error::InconsistentFilesystem);
		}
		if ofs as usize + len as usize > self.data.len() {
			return Err(crate::Error::InconsistentFilesystem);
		}
		let rv = &self.data[ofs as usize..][..len as usize];
		//::kernel::log_debug!("get_data: {:#x}+{:#x} = {:x?}", ofs, len, &rv[..32]);
		Ok(rv)
	}
}
struct InitrdInstance(::kernel::lib::mem::aref::ArefInner<Inner>);
impl crate::mount::Filesystem for InitrdInstance {
	fn root_inode(&self) -> crate::node::InodeId {
		0
	}

	fn get_node_by_inode(&self, i: crate::node::InodeId) -> Option<crate::node::Node> {
		use ::core::convert::TryFrom;
		let inodes = self.0.inodes();
		let inode = inodes.get(usize::try_from(i).ok()?)?;
		assert!(inode.ofs > 0);
		Some(match inode.ty {
		repr::NODE_TY_REGULAR => crate::node::Node::File(Box::new(NodeFile {
			parent: self.0.borrow(),
			node_id: i as u32,
			ofs: inode.ofs,
			size: inode.length,
		})),
		repr::NODE_TY_DIRECTORY => crate::node::Node::Dir(Box::new(NodeDir {
			parent: self.0.borrow(),
			node_id: i as u32,
			ofs: inode.ofs,
			size: inode.length,
		})),
		_ => return None,
		})
	}
}

struct NodeFile {
	parent: ::kernel::lib::mem::aref::ArefBorrow<Inner>,
	node_id: u32,
	ofs: u32,
	size: u32,
}
impl crate::node::NodeBase for NodeFile {
	fn get_id(&self) -> crate::node::InodeId {
		self.node_id as _
	}

	fn get_any(&self) -> &dyn core::any::Any {
		self
	}
}
impl crate::node::File for NodeFile {
	fn size(&self) -> u64 {
		self.size as u64
	}

	fn truncate(&self, _newsize: u64) -> crate::node::Result<u64> {
		Err(crate::Error::ReadOnlyFilesystem)
	}

	fn clear(&self, _ofs: u64, _size: u64) -> crate::node::Result<()> {
		Err(crate::Error::ReadOnlyFilesystem)
	}

	fn read(&self, ofs: u64, buf: &mut [u8]) -> crate::node::Result<usize> {
		let src = self.parent.get_data(self.ofs, self.size)?;
		if ofs > src.len() as u64 {
			return Err(crate::Error::InvalidParameter);
		}
		let src = &src[ofs as usize..];
		let rv = buf.len().min( src.len() );
		buf[..rv].copy_from_slice(&src[..rv]);
		Ok(rv)
	}

	fn write(&self, _ofs: u64, _buf: &[u8]) -> crate::node::Result<usize> {
		Err(crate::Error::ReadOnlyFilesystem)
	}
}

struct NodeDir {
	parent: ::kernel::lib::mem::aref::ArefBorrow<Inner>,
	node_id: u32,
	ofs: u32,
	size: u32,
}
impl NodeDir {
	fn entries(&self) -> Result<&[repr::DirEntry],super::Error> {
		let d = self.parent.get_data(self.ofs, self.size)?;
		if d.as_ptr() as usize % align_of::<repr::DirEntry>() != 0 {
			return Err(crate::Error::InconsistentFilesystem);
		}
		if d.len() as usize % size_of::<repr::DirEntry>() != 0 {
			return Err(crate::Error::InconsistentFilesystem);
		}
		// SAFE: Alignment and size checked above, data is functionally POD
		Ok(unsafe { ::core::slice::from_raw_parts(d.as_ptr() as *const _, d.len() / size_of::<repr::DirEntry>()) })
	}
}
impl crate::node::NodeBase for NodeDir {
	fn get_id(&self) -> crate::node::InodeId {
		self.node_id as _
	}

	fn get_any(&self) -> &dyn core::any::Any {
		self
	}
}
use kernel::lib::byte_str::ByteStr;
impl crate::node::Dir for NodeDir {
	fn lookup(&self, name: &ByteStr) -> crate::node::Result<crate::node::InodeId> {
		for e in self.entries()? {
			if name == trim_nuls(&e.filename) {
				return Ok(e.node as _);
			}
		}
		Err(super::Error::NotFound)
	}

	fn read(&self, start_ofs: usize, callback: &mut crate::node::ReadDirCallback) -> crate::node::Result<usize> {
		let ents = self.entries()?;
		if start_ofs >= ents.len() {
			return Ok(ents.len());
		}
		let ents_to_visit = ents.get(start_ofs..).unwrap_or(&[]);
		for (i,e) in ents_to_visit.iter().enumerate() {
			if callback(e.node as u64, &mut trim_nuls(&e.filename).iter().copied()) == false {
				return Ok(start_ofs + i + 1);
			}
		}
		Ok(start_ofs + ents_to_visit.len())
	}

	fn create(&self, _name: &ByteStr, _nodetype: crate::node::NodeType) -> crate::node::Result<crate::node::InodeId> {
		Err(super::Error::ReadOnlyFilesystem)
	}

	fn link(&self, _name: &ByteStr, _inode: &dyn crate::node::NodeBase) -> crate::node::Result<()> {
		Err(super::Error::ReadOnlyFilesystem)
	}

	fn unlink(&self, _name: &ByteStr) -> crate::node::Result<()> {
		Err(super::Error::ReadOnlyFilesystem)
	}
}


fn dump_file(data: &[u8], inodes: &[initrd_repr::Inode], name: impl ::core::fmt::Debug, inode_idx: u32, indent: usize) {
	let i = &inodes[inode_idx as usize];
	let d = &data[i.ofs as usize..][..i.length as usize];
	struct Indent(usize);
	impl ::core::fmt::Display for Indent {
		fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
			for _ in 0..self.0 {
				f.write_str("  ")?;
			}
			Ok(())
		}
	}
	::kernel::log_debug!("{}- {:?}: #{} {} @{:#x}+{:#x}", Indent(indent), name, inode_idx, i.ty, i.ofs, i.length);
	if i.ty == initrd_repr::NODE_TY_DIRECTORY {
		use initrd_repr::DirEntry;
		if d.as_ptr() as usize % align_of::<DirEntry>() != 0 {
			return;
		}
		if d.len() as usize % size_of::<DirEntry>() != 0 {
			return;
		}
		// SAFE: Alignment and size checked above, data is functionally POD
		let ents = unsafe { ::core::slice::from_raw_parts(d.as_ptr() as *const DirEntry, d.len() / size_of::<DirEntry>()) };
		for e in ents {
			dump_file(data, inodes, ::core::str::from_utf8(trim_nuls(&e.filename)), e.node, indent+1);
		}
	}
	else {
		let l = d.len().min( 32 );
		let d = &d[..l];
		::kernel::log_debug!("{}{:x?}", Indent(1+indent), d);
	}
}
