// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/metadevs/storage.rs
// - Storage (block device) subsystem
use _common::*;
use core::atomic::{AtomicUsize,ATOMIC_USIZE_INIT};
use sync::mutex::LazyMutex;
use lib::{VecMap,BTreeMap};

module_define!{Storage, [], init}

/// A unique handle to a storage volume (logical)
pub struct VolumeHandle
{
	handle: ::lib::mem::Arc<LogicalVolume>,
}

pub struct PhysicalVolumeReg
{
	idx: usize,
}

/// Helper to print out the size of a volume/size as a pretty SI base 2 number
pub struct SizePrinter(pub u64);

/// Physical volume instance provided by driver
///
/// Provides the low-level methods to manipulate the underlying storage
pub trait PhysicalVolume: Send + 'static
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
	fn read<'a>(&'a self, prio: u8, blockidx: u64, count: usize, dst: &'a mut [u8]) -> Result<Box<::async::Waiter+'a>, ()>;
	/// Writer a number of blocks to the volume
	fn write<'a>(&'a self, prio: u8, blockidx: u64, count: usize, src: &'a [u8]) -> Result<Box<::async::Waiter+'a>,()>;
	/// Erases a number of blocks from the volume
	///
	/// Erases (requests the underlying storage forget about) `count` blocks starting at `blockidx`.
	/// This is functionally equivalent to the SSD "TRIM" command.
	fn wipe(&mut self, blockidx: u64, count: usize);
}

/// Registration for a physical volume handling driver
pub trait Mapper: Send + Sync
{
	fn name(&self) -> &str;
	fn handles_pv(&self, pv: &PhysicalVolume) -> usize;
}


/// A single physical volume
struct PhysicalVolumeInfo
{
	dev: Box<PhysicalVolume>,
}
/// A single logical volume, composed of 1 or more physical blocks
struct LogicalVolume
{
	/// Logical volume name (should be unique)
	name: String,
	/// If true, a VolumeHandle exists for this volume
	is_opened: bool,
	/// Logical block size (max physical block size)
	block_size: usize,
	/// Stripe size (number of blocks), None = JBOD
	chunk_size: Option<usize>,
	/// Physical regions that compose this logical volume
	regions: Vec<PhysicalRegion>,
}
/// Physical region used by a logical volume
struct PhysicalRegion
{
	volume: usize,
	block_count: usize,	// usize to save space in average case
	first_block: u64,
}

static S_NEXT_PV_IDX: AtomicUsize = ATOMIC_USIZE_INIT;
static S_PHYSICAL_VOLUMES: LazyMutex<VecMap<usize,PhysicalVolumeInfo>> = lazymutex_init!();
static S_LOGICAL_VOLUMES: LazyMutex<VecMap<usize,LogicalVolume>> = lazymutex_init!();
static S_MAPPERS: LazyMutex<Vec<&'static Mapper>> = lazymutex_init!();

// NOTE: Should unbinding of LVs be allowed? (Yes, for volume removal)

fn init()
{
	S_PHYSICAL_VOLUMES.init( || VecMap::new() );
	S_LOGICAL_VOLUMES.init( || VecMap::new() );
	S_MAPPERS.init( || Vec::new() );
	
	// Default mapper just exposes the PV as a single LV
	//S_MAPPERS.lock().push_back(&default_mapper::Mapper);
}

/// Register a physical volume
pub fn register_pv(dev: Box<PhysicalVolume>) -> PhysicalVolumeReg
{
	log_trace!("register_pv(pv = \"{}\")", dev.name());
	let pv_id = S_NEXT_PV_IDX.fetch_add(1, ::core::atomic::Ordering::Relaxed);

	// Now that a new PV has been inserted, handlers should be informed
	let mut best_mapper: Option<&Mapper> = None;
	let mut best_mapper_level = 0;
	let mappers = S_MAPPERS.lock();
	for &mapper in mappers.iter()
	{
		let level = mapper.handles_pv(&*dev);
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
				mapper.name(), best_mapper.unwrap().name(), dev.name());
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
		todo!("Invoke mapper {} on volume {}", mapper.name(), dev.name());
	}
	
	// Wait until after checking for a handler before we add the PV to the list
	S_PHYSICAL_VOLUMES.lock().insert(pv_id, PhysicalVolumeInfo {
		dev: dev,
		});
	
	PhysicalVolumeReg { idx: pv_id }
}

/// Register a mapper with the storage subsystem
// TODO: How will it be unregistered?
pub fn register_mapper(mapper: &'static Mapper)
{
	S_MAPPERS.lock().push(mapper);
	
	// Check unbound PVs
	for (_id,pv) in S_PHYSICAL_VOLUMES.lock().iter()
	{
		let level = mapper.handles_pv(&*pv.dev);
		if level == 0
		{
			// Ignore
		}
		else
		{
			log_error!("TODO: Mapper {} wants to handle volume {}", mapper.name(), pv.dev.name());
		}
	}
}

pub fn enum_pvs() -> Vec<(usize,String)>
{
	S_PHYSICAL_VOLUMES.lock().iter().map(|(k,v)| (*k, String::from_str(v.dev.name())) ).collect()
}


pub fn enum_lvs() -> Vec<(usize,String)>
{
	S_LOGICAL_VOLUMES.lock().iter().map( |(k,v)| (*k, v.name.clone()) ).collect()
}

pub fn open_lv(idx: usize) -> Result<VolumeHandle,()>
{
	match S_LOGICAL_VOLUMES.lock().get(&idx)
	{
	Some(v) => todo!("open_lv '{}'", v.name),
	None => Err( () ),
	}
}
impl VolumeHandle
{
	pub fn block_size(&self) -> usize {
		self.handle.block_size
	}
	
	// TODO: Return a more complex type that can be incremented
	fn get_phys_block(&self, idx: u64) -> Option<(usize,u64)> {
		if let Some(size) = self.handle.chunk_size
		{
			todo!("Non JBOD logocal volumes ({} block stripe)", size);
		}
		else
		{
			let mut idx_rem = idx;
			for v in self.handle.regions.iter()
			{
				if idx_rem < v.block_count as u64 {
					return Some( (v.volume, v.first_block + idx_rem) );
				}
				else {
					idx_rem -= v.block_count as u64;
				}
			}
		}
		None
	}
	
	#[allow(dead_code)]
	pub fn read_blocks(&self, idx: u64, dst: &mut [u8]) -> Result<(),()> {
		assert!(dst.len() % self.block_size() == 0);
		
		for (block,dst) in (idx .. ).zip( dst.chunks_mut(self.block_size()) )
		{
			let (pv,ofs) = match self.get_phys_block(block) {
				Some(v) => v,
				None => return Err( () ),
				};
			try!( S_PHYSICAL_VOLUMES.lock().get(&pv).unwrap().read(ofs, dst) );
		}
		unimplemented!();
	}
}

impl PhysicalVolumeInfo
{
	fn max_blocks_per_read(&self) -> usize {
		32
	}
	
	/// Read blocks from the device
	pub fn read(&self, first: u64, dst: &mut [u8]) -> Result<usize,()>
	{
		let block_step = self.max_blocks_per_read();
		let block_size = self.dev.blocksize();
		// Read up to 'block_step' blocks in each read call
		for (blk,dst) in (first .. ).step_by(block_step as u64).zip( dst.chunks_mut( block_step * block_size ) )
		{
			let prio = 0;
			let blocks = dst.len() / block_size;
			
			// TODO: Async! (maybe return a composite read handle?)
			try!(self.dev.read(prio, blk, blocks, dst)).wait();
		}
		todo!("PhysicalVolumeInfo::read(first={},{} bytes)", first, dst.len());
	}
}

impl ::core::fmt::Display for SizePrinter
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		const THRESHOLD: u64 = 4096;	// Largest value
		if self.0 < THRESHOLD
		{
			write!(f, "{}B", self.0)
		}
		else if self.0 < THRESHOLD << 10
		{
			write!(f, "{}KiB", self.0>>10)
		}
		else if self.0 < THRESHOLD << 20
		{
			write!(f, "{}MiB", self.0>>20)
		}
		else if self.0 < THRESHOLD << 30
		{
			write!(f, "{}GiB", self.0>>40)
		}
		else //if self.0 < THRESHOLD << 40
		{
			write!(f, "{}TiB", self.0>>40)
		}
	}
}

// vim: ft=rust
