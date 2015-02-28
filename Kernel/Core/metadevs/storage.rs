// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/metadevs/storage.rs
// - Storage (block device) subsystem
use _common::*;
use core::atomic::{AtomicUint,ATOMIC_UINT_INIT};
use sync::mutex::LazyMutex;
use async::{ReadHandle,WriteHandle};
use lib::VecMap;

module_define!{Storage, [], init}

/// A unique handle to a storage volume (logical)
pub struct VolumeHandle
{
	lv_idx: usize,
}

pub struct PhysicalVolumeReg
{
	idx: usize,
}

/// Physical volume instance provided by driver
///
/// Provides the low-level methods to manipulate the underlying storage
pub trait PhysicalVolume
{
	/// Returns the volume name (must be unique to the system)
	fn name(&self) -> &str;	// Local lifetime string
	/// Returns the size of a filesystem block, must be a power of two >512
	fn blocksize(&self) -> usize;
	/// Returns the number of blocks in this volume (i.e. the capacity)
	fn capacity(&self) -> u64;
	
	/// Reads a number of blocks from the volume into the provided buffer
	///
	/// Reads `count` blocks starting with `blockidx` into the buffer `dst` (which will/should
	/// be the size of `count` blocks). The read is performed with the provided priority, where
	/// 0 is higest, and 255 is lowest.
	fn read<'a>(&'a self, prio: u8, blockidx: u64, count: usize, dst: &'a mut [u8]) -> Result<ReadHandle<'a,'a>, ()>;
	/// Writer a number of blocks to the volume
	fn write<'a>(&'a self, prio: u8, blockidx: u64, count: usize, src: &'a [u8]) -> Result<WriteHandle<'a,'a>,()>;
	/// Erases a number of blocks from the volume
	///
	/// Erases (requests the underlying storage forget about) `count` blocks starting at `blockidx`.
	/// This is functionally equivalent to the SSD "TRIM" command.
	fn wipe(&mut self, blockidx: u64, count: usize);
}

/// Registration for a physical volume handling driver
trait Mapper: Send + Sync
{
	fn name(&self) -> &str;
	fn handles_pv(&self, pv: &PhysicalVolume) -> usize;
}

/// A single logical volume, composed of 1 or more physical blocks
struct LogicalVolume
{
	block_size: usize,	///< Logical block size (max physical block size)
	region_size: Option<usize>,	///< Number of bytes in each physical region, None = JBOD
	regions: Vec<PhysicalRegion>,
}
/// Physical region used by a logical volume
struct PhysicalRegion
{
	volume: usize,
	block_count: usize,	// usize to save space in average case
	first_block: u64,
}

static s_next_pv_idx: AtomicUint = ATOMIC_UINT_INIT;
static s_physical_volumes: LazyMutex<VecMap<usize,Box<PhysicalVolume+Send>>> = lazymutex_init!();
static s_logical_volumes: LazyMutex<VecMap<usize,LogicalVolume>> = lazymutex_init!();
static s_mappers: LazyMutex<Vec<&'static Mapper>> = lazymutex_init!();

// TODO: Maintain a set of registered volumes. Mappers can bind onto a volume and register new LVs
// TODO: Maintain set of active mappings (set of PVs -> set of LVs)
// NOTE: Should unbinding of LVs be allowed? (Yes, for volume removal)

fn init()
{
	s_physical_volumes.init( || VecMap::new() );
}

pub fn register_pv(pv: Box<PhysicalVolume+Send>) -> PhysicalVolumeReg
{
	let pv_id = s_next_pv_idx.fetch_add(1, ::core::atomic::Ordering::Relaxed);

	// Now that a new PV has been inserted, handlers should be informed
	let mut best_mapper: Option<&Mapper> = None;
	let mut best_mapper_level = 0;
	let mappers = s_mappers.lock();
	for &mapper in mappers.iter()
	{
		let level = mapper.handles_pv(&*pv);
		if level == 0
		{
			// Ignore (doesn't handle)
		}
		else if level < best_mapper_level
		{
			// Ignore (weaker handle)
		}
		else if level == best_mapper_level
		{
			// Fight!
			log_warning!("LV Mappers {} and {} are fighting over {}",
				mapper.name(), best_mapper.unwrap().name(), pv.name());
		}
		else
		{
			best_mapper = Some(mapper);
			best_mapper_level = level;
		}
	}
	if let Some(mapper) = best_mapper
	{
		// Poke mapper
		unimplemented!();
	}
	
	// Wait until after checking for a handler before we add the PV to the list
	s_physical_volumes.lock().insert(pv_id, pv);
	
	PhysicalVolumeReg { idx: pv_id }
}

/// Function called when a new volume is registered (physical or logical)
fn new_volume(volidx: usize)
{
}

pub fn enum_pvs() -> Vec<(usize,String)>
{
	s_physical_volumes.lock().iter().map(|(k,v)| (*k, String::from_str(v.name())) ).collect()
}

// vim: ft=rust
