// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/metadevs/storage.rs
// - Storage (block device) subsystem
use prelude::*;
use core::atomic::{AtomicUsize,ATOMIC_USIZE_INIT};
use sync::mutex::LazyMutex;
use lib::{VecMap};
use lib::mem::Arc;

module_define!{Storage, [], init}

pub type AsyncIoResult<'a, T> = ::async::BoxAsyncResult<'a, T, IoError>;

/// A unique handle to a storage volume (logical)
pub struct VolumeHandle
{
	handle: ::lib::mem::Arc<LogicalVolume>,
}

/// Physical volume registration (PV will be deregistered when this handle is dropped)
/// 
// TODO: What is the behavior when this PV still has LVs (open LVs too?). Just waiting will not
// be the correct behavior.
pub struct PhysicalVolumeReg
{
	idx: usize,
}

/// Helper to print out the size of a volume/size as a pretty SI base 2 number
pub struct SizePrinter(pub u64);

/// Block-level input-output error
#[derive(Debug,Copy,Clone)]
pub enum IoError
{
	BadAddr,
	InvalidParameter,
	Timeout,
	BadBlock,
	ReadOnly,
	NoMedium,
	Unknown(&'static str),
}

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
	fn capacity(&self) -> Option<u64>;
	
	/// Reads a number of blocks from the volume into the provided buffer
	///
	/// Reads `count` blocks starting with `blockidx` into the buffer `dst` (which will/should
	/// be the size of `count` blocks). The read is performed with the provided priority, where
	/// 0 is higest, and 255 is lowest.
	fn read<'a>(&'a self, prio: u8, blockidx: u64, count: usize, dst: &'a mut [u8]) -> AsyncIoResult<'a,()>;
	/// Writer a number of blocks to the volume
	fn write<'a>(&'a self, prio: u8, blockidx: u64, count: usize, src: &'a [u8]) -> AsyncIoResult<'a,()>;
	/// Erases a number of blocks from the volume
	///
	/// Erases (requests the underlying storage forget about) `count` blocks starting at `blockidx`.
	/// This is functionally equivalent to the SSD "TRIM" command.
	fn wipe<'a>(&'a self, blockidx: u64, count: usize) -> AsyncIoResult<'a,()>;
}

/// Registration for a physical volume handling driver
pub trait Mapper: Send + Sync
{
	/// Return the "name" of this mapper (e.g. mbr, gpt)
	fn name(&self) -> &str;
	/// Returns the binding strength of this mapper.
	///
	/// Lower values are weaker handles, 0 means unhandled.
	/// Typical values are: 1=MBR, 2=GPT, 3=LVM etc
	fn handles_pv(&self, pv: &PhysicalVolume) -> usize;
	
	/// Enumerate volumes
	fn enum_volumes(&self, pv: &PhysicalVolume, f: &mut FnMut(String, u64, u64));
}


/// A single physical volume
struct PhysicalVolumeInfo
{
	dev: Box<PhysicalVolume>,
	mapper: Option<(usize,&'static Mapper)>,
}
/// A single logical volume, composed of 1 or more physical blocks
#[derive(Default)]
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
static S_NEXT_LV_IDX: AtomicUsize = ATOMIC_USIZE_INIT;
static S_LOGICAL_VOLUMES: LazyMutex<VecMap<usize,Arc<LogicalVolume>>> = lazymutex_init!();
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
	// - Only try to resolve a mapper if there's media in the drive
	if dev.capacity().is_some()
	{
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
	}
	
	// Wait until after checking for a handler before we add the PV to the list
	S_PHYSICAL_VOLUMES.lock().insert(pv_id, PhysicalVolumeInfo {
		dev: dev,
		mapper: None,
		});
	
	if let Some(mapper) = best_mapper {
		apply_mapper_to_pv(mapper, best_mapper_level, pv_id, S_PHYSICAL_VOLUMES.lock().get_mut(&pv_id).unwrap())
	}
	else {
		// Apply the fallback (full volume) mapper
		apply_mapper_to_pv(&default_mapper::S_MAPPER, 0, pv_id, S_PHYSICAL_VOLUMES.lock().get_mut(&pv_id).unwrap())
	}
	
	PhysicalVolumeReg { idx: pv_id }
}

/// Register a mapper with the storage subsystem
// TODO: How will it be unregistered. Requires a mapper handle that ensures that the mapper is unregistered when the relevant
// module is unloaded.
// TODO: In the current model, mappers can be unloaded without needing the volumes to be unmounted, but a possible
// extension is to allow the mapper to handle logical->physical itself.
pub fn register_mapper(mapper: &'static Mapper)
{
	S_MAPPERS.lock().push(mapper);
	
	// Check unbound PVs
	for (&id,pv) in S_PHYSICAL_VOLUMES.lock().iter_mut()
	{
		if pv.dev.capacity().is_none() {
			// No media, skip
			continue ;
		}
		let level = mapper.handles_pv(&*pv.dev);
		if level == 0
		{
			// Ignore
		}
		else
		{
			if let Some( (lvl, _other) ) = pv.mapper
			{
				if lvl == level {
					// fight
				}
				else if lvl > level {
					// Already better
				}
				else {
					// Replace
					apply_mapper_to_pv(mapper, level, id, pv);
				}
			}
			else
			{
				apply_mapper_to_pv(mapper, level, id, pv);
			}
		}
	}
}

/// Apply the passed mapper to the provided physical volume
fn apply_mapper_to_pv(mapper: &'static Mapper, level: usize, pv_id: usize, pvi: &mut PhysicalVolumeInfo)
{
	// - Can't compare fat raw pointers (ICE, #23888)
	//assert!(level > 0 || mapper as *const _ == &default_mapper::S_MAPPER as *const _);
	
	// TODO: LOCK THE PVI
	// 1. Determine if a previous mapper was controlling this volume
	if let Some(..) = pvi.mapper
	{
		// Attempt to remove these mappings if possible
		// > This means iterating the LV list (locked) and first checking if all
		//   from this PV are not mounted, then removing them.
		let mut lh = S_LOGICAL_VOLUMES.lock();
		let keys: Vec<usize> = {
			// - Count how many LVs using this PV are mounted
			let num_mounted = lh.iter()
				.filter( |&(_,lv)| lv.regions.iter().any(|r| r.volume == pv_id) )
				.filter(|&(_,lv)| lv.is_opened)
				.count();
			if num_mounted > 0 {
				log_notice!("{}LVs using PV #{} {} are mounted, not updating mapping", num_mounted, pv_id, pvi.dev.name() );
				return ;
			}
			// > If none are mounted, then remove the mappings
			lh.iter()
				.filter( |&(_,lv)| lv.regions.iter().any(|r| r.volume == pv_id) )
				.map(|(&i,_)| i)
				.collect()
			};
		log_debug!("Removing {} LVs", keys.len());
		for k in keys {
			lh.remove(&k);
		}
		pvi.mapper = None;
	}
	// 2. Bind this new mapper to the volume
	// - Save the mapper
	pvi.mapper = Some( (level, mapper) );
	// - Enumerate volumes
	//  TODO: Support more complex volume types
	mapper.enum_volumes(&*pvi.dev, &mut |name, base, len| {
		new_simple_lv(name, pv_id, pvi.dev.blocksize(), base, len);
		});
}
fn new_simple_lv(name: String, pv_id: usize, block_size: usize, base: u64, size: u64)
{
	let lvidx = S_NEXT_LV_IDX.fetch_add(1, ::core::atomic::Ordering::Relaxed);
	
	assert!(size <= !0usize as u64);
	let lv = Arc::new( LogicalVolume {
		name: name,
		is_opened: false,
		block_size: block_size,
		chunk_size: None,
		regions: vec![ PhysicalRegion{ volume: pv_id, block_count: size as usize, first_block: base } ],
		} );
	
	log_log!("Logical Volume: {} {}", lv.name, SizePrinter(size*block_size as u64));
	
	// Add to global list
	{
		let mut lh = S_LOGICAL_VOLUMES.lock();
		lh.insert(lvidx, lv);
	}
	// TODO: Inform something of the new LV
}

/// Enumerate present physical volumes (returning both the identifier and name)
pub fn enum_pvs() -> Vec<(usize,String)>
{
	S_PHYSICAL_VOLUMES.lock().iter().map(|(k,v)| (*k, String::from_str(v.dev.name())) ).collect()
}


/// Enumerate present logical volumes (returning both the identifier and name)
pub fn enum_lvs() -> Vec<(usize,String)>
{
	S_LOGICAL_VOLUMES.lock().iter().map( |(k,v)| (*k, v.name.clone()) ).collect()
}

#[derive(Debug)]
pub enum VolOpenError
{
	NotFound,
	Locked,
}
impl_fmt!{
	Display(self,f) for VolOpenError {
		write!(f, "{}",
			match self
			{
			&VolOpenError::NotFound => "No such logical volume",
			&VolOpenError::Locked => "Logical volume already open",
			})
	}
}

impl VolumeHandle
{
	pub fn new_ramdisk(_count: usize) -> VolumeHandle {
		VolumeHandle {
			handle: Arc::new(LogicalVolume::default())
		}
	}
	/// Acquire an unique handle to a logical volume
	pub fn open_idx(idx: usize) -> Result<VolumeHandle,VolOpenError>
	{
		match S_LOGICAL_VOLUMES.lock().get(&idx)
		{
		Some(v) => todo!("open_lv '{}'", v.name),
		None => Err( VolOpenError::NotFound ),
		}
	}
	pub fn open_named(name: &str) -> Result<VolumeHandle,VolOpenError> {
		match S_LOGICAL_VOLUMES.lock().iter_mut().find(|&(_, ref v)| v.name == name)
		{
		Some((_,v)) => {
			if ::lib::mem::arc::get_mut(v).is_some() {
				Ok( VolumeHandle { handle: v.clone() } )
			}
			else {
				Err( VolOpenError::Locked )
			}
			},
		None => Err( VolOpenError::NotFound ),
		}
	}
	
	pub fn block_size(&self) -> usize {
		self.handle.block_size
	}
	
	// TODO: Return a more complex type that can be incremented
	// Returns: VolIdx, Block, Count
	fn get_phys_block(&self, idx: u64, count: usize) -> Option<(usize,u64,usize)> {
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
					let ret_count = ::core::cmp::min(
						v.block_count as u64 - idx_rem,
						count as u64
						) as usize;
					return Some( (v.volume, v.first_block + idx_rem, ret_count) );
				}
				else {
					idx_rem -= v.block_count as u64;
				}
			}
		}
		None
	}
	
	#[allow(dead_code)]
	/// Read a series of blocks from the volume into the provided buffer.
	/// 
	/// The buffer must be a multiple of the logical block size
	pub fn read_blocks(&self, idx: u64, dst: &mut [u8]) -> Result<(),IoError> {
		log_trace!("VolumeHandle::read_blocks(idx={}, dst={{len={}}})", idx, dst.len());
		if dst.len() % self.block_size() != 0 {
			log_warning!("Read size {} not a multiple of {} bytes", dst.len(), self.block_size());
			return Err( IoError::InvalidParameter );
		}
		
		let mut rem = dst.len() / self.block_size();
		let mut blk = 0;
		while rem > 0
		{
			let (pv, ofs, count) = match self.get_phys_block(idx + blk as u64, rem) {
				Some(v) => v,
				None => return Err( IoError::BadAddr ),
				};
			log_trace!("- PV{} {} + {}", pv, ofs, count);
			assert!(count <= rem);
			let bofs = blk as usize * self.block_size();
			let dst = &mut dst[bofs .. bofs + count * self.block_size()];
			try!( S_PHYSICAL_VOLUMES.lock().get(&pv).unwrap().read(ofs, dst) );
			blk += count;
			rem -= count;
		}
		Ok( () )
	}
}

impl PhysicalVolumeInfo
{
	fn max_blocks_per_read(&self) -> usize {
		32
	}
	
	/// Read blocks from the device
	pub fn read(&self, first: u64, dst: &mut [u8]) -> Result<usize,IoError>
	{
		log_trace!("PhysicalVolumeInfo::read(first={},{} bytes)", first, dst.len());
		let block_step = self.max_blocks_per_read();
		let block_size = self.dev.blocksize();
		// Read up to 'block_step' blocks in each read call
		{
			let iter_ids  = (first .. ).step_by(block_step as u64);
			let iter_bufs = dst.chunks_mut( block_step * block_size );
			for (blk_id,buf) in iter_ids.zip( iter_bufs )
			{
				let prio = 0;
				let blocks = buf.len() / block_size;
				
				// TODO: Async! (maybe return a composite read handle?)
				self.dev.read(prio, blk_id, blocks, buf).wait().unwrap()
			}
		}
		Ok(dst.len()/block_size)
	}
}

impl ::core::ops::Drop for PhysicalVolumeReg
{
	fn drop(&mut self)
	{
		todo!("PhysicalVolumeReg::drop idx={}", self.idx);
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

mod default_mapper
{
	use prelude::*;
	
	pub struct Mapper;
	
	pub static S_MAPPER: Mapper = Mapper;
	
	impl ::metadevs::storage::Mapper for Mapper {
		fn name(&self) -> &str { "fallback" }
		fn handles_pv(&self, _pv: &::metadevs::storage::PhysicalVolume) -> usize {
			// The fallback mapper never explicitly handles
			0
		}
		fn enum_volumes(&self, pv: &::metadevs::storage::PhysicalVolume, new_volume_cb: &mut FnMut(String, u64, u64)) {
			if let Some(cap) = pv.capacity() {
				new_volume_cb(format!("{}w", pv.name()), 0, cap );
			}
		}
	}
}

// vim: ft=rust
