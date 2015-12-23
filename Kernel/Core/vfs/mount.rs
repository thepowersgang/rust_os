// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/mount.rs
//! Mountpoint managment
use prelude::*;
use super::path::{Path,PathBuf};
use super::node::{InodeId,Node,CacheHandle};
use sync::RwLock;
use lib::{LazyStatic,SparseVec,VecMap};

use metadevs::storage::VolumeHandle;

/// A handle to a mounted filesystem
pub struct Handle(usize);

struct MountedVolume
{
	mountpoint_node: CacheHandle,
	fs: Box<Filesystem>,
}


/// Filesystem instance trait (i.e. the instance)
pub trait Filesystem:
	Send + Sync
{
	fn root_inode(&self) -> InodeId;
	fn get_node_by_inode(&self, InodeId) -> Option<Node>;
}

/// Filesystem instance trait
pub trait Driver:
	Send + Sync
{
	fn detect(&self, vol: &VolumeHandle) -> super::Result<usize>;
	fn mount(&self, vol: VolumeHandle) -> super::Result<Box<Filesystem>>;
}

pub struct DriverRegistration(&'static str);

/// Known drivers
static S_DRIVERS: LazyStatic<RwLock< VecMap<&'static str, &'static Driver> >> = lazystatic_init!();
/// Mounted volumes
static S_VOLUMES: LazyStatic<RwLock< SparseVec<MountedVolume> >> = lazystatic_init!();
static S_ROOT_VOLUME: RwLock<Option<Box<Filesystem>>> = RwLock::new(None);

pub fn init()
{
	// SAFE: Running in a single-threaded context
	unsafe {
		S_DRIVERS.prep( || Default::default() );
		S_VOLUMES.prep( || Default::default() );
	}
}

/// Mount a volume at the provided location
// TODO: Parse options
pub fn mount(location: &Path, vol: VolumeHandle, fs: &str, _options: &[&str]) -> Result<(),MountError>
{
	let drivers = S_DRIVERS.read();
	// 1. (maybe) detect filesystem
	let driver = if fs == "" {
			match drivers.iter()
				.filter_map(|(n,fs)| fs.detect(&vol).ok().map(|r| (r, n, fs)))
				.max_by_key(|&(l,_,_)| l)
			{
			Some((0,_,_)) => return Err(MountError::NoHandler),
			Some((_,_name,fs)) => fs,
			None => return Err(MountError::NoHandler),
			}
		}
		else {
			match drivers.get(fs)
			{
			Some(d) => d,
			None => return Err(MountError::UnknownFilesystem),
			}
		};
	
	if location == Path::new("/")
	{
		let fs: Box<_> = match driver.mount(vol)
			{
			Ok(v) => v,
			Err(_) => return Err(MountError::CallFailed),
			};
		let mut lh = S_ROOT_VOLUME.write();
		if lh.is_some() {
			todo!("Remount /");
		}
		*lh = Some(fs);
	}
	else
	{
		// 2. Acquire mountpoint
		let nh = match CacheHandle::from_path(location)
			{
			Ok(nh) => nh,
			Err(_) => return Err(MountError::InvalidMountpoint),
			};
		if ! nh.is_dir() {
			return Err(MountError::InvalidMountpoint);
		}
		if nh.is_mountpoint() {
			return Err(MountError::MountpointUsed);
		}
		
		// 3. Mount and register volume
		let fs = match driver.mount(vol)
			{
			Ok(v) => v,
			Err(_) => return Err(MountError::CallFailed),
			};
		let vidx = S_VOLUMES.write().insert(MountedVolume {
			mountpoint_node: nh.clone(),
			fs: fs,
			});
		
		// 4. Bind to mountpoint
		if nh.mount( vidx + 1 ) == false {
			S_VOLUMES.write().remove(vidx);
			return Err(MountError::MountpointUsed);
		}
	}

	Ok( () )
}
#[derive(Debug)]
pub enum MountError
{
	UnknownFilesystem,
	NoHandler,
	InvalidMountpoint,
	MountpointUsed,
	CallFailed,
}
impl_fmt! {
	Display(self,f) for MountError {
		write!(f, "{}", match self
			{
			&MountError::UnknownFilesystem => "Filesystem driver not found",
			&MountError::NoHandler => "No registered filesystem driver handles this volume",
			&MountError::InvalidMountpoint => "The specified mountpoint was invalid",
			&MountError::MountpointUsed => "The specified mountpoint was already used",
			&MountError::CallFailed => "Driver's mount call failed",
			})
	}
}


impl DriverRegistration
{
	pub fn new(name: &'static str, fs: &'static Driver) -> Option<DriverRegistration> {
		match S_DRIVERS.write().entry(name)
		{
		::lib::vec_map::Entry::Vacant(e) => {
			e.insert(fs);
			Some(DriverRegistration(name))
			},
		::lib::vec_map::Entry::Occupied(_) => None,
		}
	}
}

impl Handle
{
	pub fn from_id(id: usize) -> Handle {
		if id == 0 {
			Handle(0)
		}
		else {
			if ! S_VOLUMES.read().get(id-1).is_some() {
				panic!("Handle::from_id - ID {} not valid", id);
			}
			Handle(id)
		}
	}
	
	pub fn id(&self) -> usize {
		self.0
	}
	pub fn root_inode(&self) -> InodeId {
		self.with_fs(|fs| fs.root_inode())
	}
	
	pub fn get_node(&self, id: InodeId) -> Option<Node> {
		self.with_fs(|fs| fs.get_node_by_inode(id))
	}

	fn with_fs<R, F: FnOnce(&Filesystem)->R>(&self, f: F) -> R {
		if self.0 == 0 {
			f(&**S_ROOT_VOLUME.read().as_ref().unwrap())
		}
		else {
			f(&*S_VOLUMES.read().get(self.0 - 1).unwrap().fs)
		}
	}
}

