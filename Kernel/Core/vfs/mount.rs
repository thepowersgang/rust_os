// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/vfs/mount.rs
//! Mountpoint managment
use prelude::*;
use super::path::{Path,PathBuf};
use super::node::{InodeId,Node};
use sync::RwLock;
use lib::{LazyStatic,SparseVec,VecMap};

use metadevs::storage::VolumeHandle;

/// A handle to a mounted filesystem
pub struct Handle(usize);

struct Mountpoint
{
	path: PathBuf,
	volume_id: usize,
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
/// Active mountpoints
static S_MOUNTS: LazyStatic<RwLock< Vec<Mountpoint> >> = lazystatic_init!();
/// Mounted volumes
static S_VOLUMES: LazyStatic<RwLock< SparseVec<Box<Filesystem>> >> = lazystatic_init!();

pub fn init()
{
	// SAFE: Running in a single-threaded context
	unsafe {
		S_DRIVERS.prep( || Default::default() );
		S_MOUNTS.prep( || Default::default() );
		S_VOLUMES.prep( || Default::default() );
	}
}

/// Mount a volume at the provided location
pub fn mount(location: &Path, vol: VolumeHandle, fs: &str, options: &[&str]) -> Result<(),MountError>
{
	let drivers = S_DRIVERS.read();
	// 1. (maybe) detect filesystem
	let driver = if fs == "" {
			match drivers.iter()
				.filter_map(|(n,fs)| fs.detect(&vol).ok().map(|r| (r, n, fs)))
				.max_by(|&(l,_,_)| l)
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
	// 2. Check that mountpoint is valid
	let mut mountpoints = S_MOUNTS.write();
	// - Mount list is sorted by path, allows simpler logic in lookup
	let idx = match mountpoints.binary_search_by(|a| Ord::cmp(&*a.path, location))
		{
		Ok(_) => return Err(MountError::MountpointUsed),
		Err(i) => i,
		};
	
	
	// 3. Mount and register volume
	let fs = match driver.mount(vol)
		{
		Ok(v) => v,
		Err(_) => return Err(MountError::CallFailed),
		};
	let vidx = S_VOLUMES.write().insert(fs);
	
	mountpoints.insert(idx, Mountpoint {
		path: PathBuf::from(location),
		volume_id: vidx,
		});
	Ok( () )
}
pub enum MountError
{
	UnknownFilesystem,
	NoHandler,
	MountpointUsed,
	CallFailed,
}
impl_fmt! {
	Debug(self,f) for MountError {
		write!(f, "{}", match self
			{
			&MountError::UnknownFilesystem => "Filesystem driver not found",
			&MountError::NoHandler => "No registerd filesystem driver handles this volume",
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
	pub fn for_path(path: &Path) -> Result<(Handle,&Path),super::Error> {
		log_trace!("Handle::for_path({:?})", path);
		if !path.is_absolute() {
			return Err(super::Error::Unknown("Path not absolute"));
		}
		if !path.is_normalised() {
			return Err(super::Error::Unknown("Path not normalised"));
		}
		let lh = S_MOUNTS.read();
		// Work backwards until a prefix match is found
		// - The mount list is sorted, which means that longer items are later in the list
		for ent in lh.iter().rev()
		{
			if let Some(tail) = path.starts_with( &ent.path ) {
				log_debug!("Return {}'{:?}', tail={:?}", ent.volume_id, ent.path, tail);
				return Ok( (Handle(ent.volume_id), tail) );
			}
		}
		Err( super::Error::Unknown("/ mount is missing") )
	}
	pub fn from_id(id: usize) -> Handle {
		assert!(S_VOLUMES.read().get(id).is_some());
		Handle(id)
	}
	
	pub fn id(&self) -> usize {
		self.0
	}
	pub fn root_inode(&self) -> InodeId {
		S_VOLUMES.read().get(self.0).unwrap().root_inode()
	}
	
	pub fn get_node(&self, id: InodeId) -> Option<Node> {
		S_VOLUMES.read().get(self.0).unwrap().get_node_by_inode(id)
	}
}

