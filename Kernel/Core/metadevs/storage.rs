// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/metadevs/storage.rs
// - Storage (block device) subsystem
use crate::prelude::*;
use core::sync::atomic::AtomicUsize;
use crate::sync::mutex::LazyMutex;
use crate::lib::VecMap;
use crate::lib::mem::Arc;

module_define!{Storage, [], init}

pub type AsyncIoResult<'a, T> = ::core::pin::Pin<Box<dyn ::core::future::Future<Output=Result<T, IoError>> + 'a>>;

/// A unique handle to a storage volume (logical)
pub struct VolumeHandle
{
	handle: crate::lib::mem::Arc<LogicalVolume>,
	// TODO: Store within this a single block cache? Or store on the LV?
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

/// Mutable/Immutable data pointer, encoded as host-relative (Send = immutable data)
pub enum DataPtr<'a>
{
	Send(&'a [u8]),
	Recv(&'a mut [u8]),
}
impl<'a> DataPtr<'a> {
	pub fn as_slice(&self) -> &[u8] {
		match self
		{
		&DataPtr::Send(p) => p,
		&DataPtr::Recv(ref p) => p,
		}
	}
	pub fn len(&self) -> usize {
		self.as_slice().len()
	}
	pub fn is_send(&self) -> bool {
		match self
		{
		&DataPtr::Send(_) => true,
		&DataPtr::Recv(_) => false,
		}
	}
}
impl<'a> ::core::fmt::Debug for DataPtr<'a> {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		match self
		{
		&DataPtr::Send(p) => write!(f, "Send({:p}+{})", p.as_ptr(), p.len()),
		&DataPtr::Recv(ref p) => write!(f, "Recv(mut {:p}+{})", p.as_ptr(), p.len()),
		}
	}
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
	/// 0 is highest, and 255 is lowest.
	///
	/// The yielded return value is the number of blocks that were written in this request (which
	/// can be less than `count`, if the underlying medium has a maximum transfer size).
	fn read<'a>(&'a self, prio: u8, blockidx: u64, count: usize, dst: &'a mut [u8]) -> AsyncIoResult<'a, usize>;
	/// Writer a number of blocks to the volume
	fn write<'a>(&'a self, prio: u8, blockidx: u64, count: usize, src: &'a [u8]) -> AsyncIoResult<'a, usize>;
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
	fn handles_pv(&self, pv: &dyn PhysicalVolume) -> Result<usize,IoError>;
	
	/// Enumerate volumes
	fn enum_volumes(&self, pv: &dyn PhysicalVolume, f: &mut dyn FnMut(String, u64, u64)) -> Result<(),IoError>;
}


/// A single physical volume
struct PhysicalVolumeInfo
{
	dev: Box<dyn PhysicalVolume>,
	mapper: Option<(usize,&'static dyn Mapper)>,
}
/// A single logical volume, composed of 1 or more physical blocks
#[derive(Default)]
struct LogicalVolume
{
	/// LV Index, should be equal to the index in the VecMap
	index: usize,
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

static S_NEXT_PV_IDX: AtomicUsize = AtomicUsize::new(0);
static S_PHYSICAL_VOLUMES: LazyMutex<VecMap<usize,PhysicalVolumeInfo>> = lazymutex_init!();
static S_NEXT_LV_IDX: AtomicUsize = AtomicUsize::new(0);
static S_LOGICAL_VOLUMES: LazyMutex<VecMap<usize,Arc<LogicalVolume>>> = lazymutex_init!();
static S_MAPPERS: LazyMutex<Vec<&'static dyn Mapper>> = lazymutex_init!();

// NOTE: Should unbinding of LVs be allowed? (Yes, for volume removal)

fn init()
{
	S_PHYSICAL_VOLUMES.init( || VecMap::new() );
	S_LOGICAL_VOLUMES.init( || VecMap::new() );
	S_MAPPERS.init( || Vec::new() );
	
	// Default mapper just exposes the PV as a single LV
	//S_MAPPERS.lock().push_back(&default_mapper::Mapper);
	::core::mem::forget( register_pv(Box::new(null_volume::NullVolume)) );
}

/// Register a physical volume
pub fn register_pv(dev: Box<dyn PhysicalVolume>) -> PhysicalVolumeReg
{
	log_trace!("register_pv(pv = \"{}\")", dev.name());
	let pv_id = S_NEXT_PV_IDX.fetch_add(1, ::core::sync::atomic::Ordering::Relaxed);

	// Now that a new PV has been inserted, handlers should be informed
	let mut best_mapper: Option<&dyn Mapper> = None;
	let mut best_mapper_level = 0;
	// - Only try to resolve a mapper if there's media in the drive
	if dev.capacity().is_some()
	{
		let mappers = S_MAPPERS.lock();
		for &mapper in mappers.iter()
		{
			match mapper.handles_pv(&*dev)
			{
			Err(e) => log_error!("IO Error in mapper detection: {:?}", e),
			Ok(0) => {},	// Ignore (doesn't handle)
			Ok(level) =>
				if level < best_mapper_level
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
				},
			}
		}
	}
	
	// Wait until after checking for a handler before we add the PV to the list
	S_PHYSICAL_VOLUMES.lock().insert(pv_id, PhysicalVolumeInfo {
		dev: dev,
		mapper: None,
		});
	
	if let Some(mapper) = best_mapper {
		apply_mapper_to_pv(mapper, best_mapper_level, pv_id, S_PHYSICAL_VOLUMES.lock().get_mut(&pv_id).unwrap());
	}
	else {
	}

	// Apply the fallback (full volume) mapper - always present
	{
		let mapper = &default_mapper::S_MAPPER;
		let mut lh = S_PHYSICAL_VOLUMES.lock();
		let pvi = lh.get_mut(&pv_id).unwrap();
		match mapper.enum_volumes(&*pvi.dev, &mut |name, base, len| {
			new_simple_lv(name, pv_id, pvi.dev.blocksize(), base, len);
			})
		{
		Err(e) => log_error!("IO Error while enumerating {}: {:?}", pvi.dev.name(), e),
		Ok(_) => {},
		}
	}
	
	PhysicalVolumeReg { idx: pv_id }
}

/// Register a mapper with the storage subsystem
// TODO: How will it be unregistered. Requires a mapper handle that ensures that the mapper is unregistered when the relevant
// module is unloaded.
// TODO: In the current model, mappers can be unloaded without needing the volumes to be unmounted, but a possible
// extension is to allow the mapper to handle logical->physical itself.
pub fn register_mapper(mapper: &'static dyn Mapper)
{
	S_MAPPERS.lock().push(mapper);
	
	// Check unbound PVs
	for (&id,pv) in S_PHYSICAL_VOLUMES.lock().iter_mut()
	{
		if pv.dev.capacity().is_none() {
			// No media, skip
			continue ;
		}
		match mapper.handles_pv(&*pv.dev)
		{
		Err(e) => log_error!("Error checking PV{}: {:?}", pv.dev.name(), e),
		Ok(0) => {},	// Ignore
		Ok(level) => 
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
			},
		}
	}
}

/// Apply the passed mapper to the provided physical volume
fn apply_mapper_to_pv(mapper: &'static dyn Mapper, level: usize, pv_id: usize, pvi: &mut PhysicalVolumeInfo)
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
	match mapper.enum_volumes(&*pvi.dev, &mut |name, base, len| {
		new_simple_lv(name, pv_id, pvi.dev.blocksize(), base, len);
		})
	{
	Err(e) => log_error!("IO Error while enumerating {}: {:?}", pvi.dev.name(), e),
	Ok(_) => {},
	}
}
fn new_simple_lv(name: String, pv_id: usize, block_size: usize, base: u64, size: u64)
{
	let lvidx = S_NEXT_LV_IDX.fetch_add(1, ::core::sync::atomic::Ordering::Relaxed);
	
	assert!(size <= !0usize as u64);
	let lv = Arc::new( LogicalVolume {
		index: lvidx,
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
	S_PHYSICAL_VOLUMES.lock().iter().map(|(k,v)| (*k, v.dev.name().to_owned()) ).collect()
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
	/// Acquire an unique handle to a logical volume
	pub fn open_named(name: &str) -> Result<VolumeHandle,VolOpenError> {
		match S_LOGICAL_VOLUMES.lock().iter_mut().find(|&(_, ref v)| v.name == name)
		{
		Some((_,v)) => {
			if Arc::get_mut(v).is_some() {
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

	pub fn idx(&self) -> usize {
		self.handle.index
	}
	pub fn name(&self) -> &str {
		&self.handle.name
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
			// HACK! Allow storage drivers to deliver metadata in the -1 block
			if idx == !0 && count == 1 && self.handle.regions.len() == 1 && self.handle.regions[0].first_block == 0 {
				let v = &self.handle.regions[0];
				return Some( (v.volume, !0, 1 ));
			}
			
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
	
	/// Read a series of blocks from the volume into the provided buffer.
	/// 
	/// The buffer must be a multiple of the logical block size
	pub async fn read_blocks(&self, idx: u64, dst: &mut [u8]) -> Result<(),IoError>
	{
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
				None => {
					log_warning!("VolumeHandle::read_blocks - Block id {} is invalid", idx + blk as u64);
					return Err( IoError::BadAddr )
					},
				};
			log_trace!("- PV{} {} + {}", pv, ofs, count);
			assert!(count <= rem);
			let bofs = blk as usize * self.block_size();
			let dst = &mut dst[bofs .. bofs + count * self.block_size()];
			S_PHYSICAL_VOLUMES.lock().get(&pv).expect("Volume missing").read(ofs, dst).await?;
			blk += count;
			rem -= count;
		}
		Ok( () )
	}

	pub async fn write_blocks(&self, idx: u64, dst: &[u8]) -> Result<(),IoError>
	{
		log_trace!("VolumeHandle::write_blocks(idx={}, dst={{len={}}})", idx, dst.len());
		if dst.len() % self.block_size() != 0 {
			log_warning!("Write size {} not a multiple of {} bytes", dst.len(), self.block_size());
			return Err( IoError::InvalidParameter );
		}
		
		let mut rem = dst.len() / self.block_size();
		let mut blk = 0;
		while rem > 0
		{
			let (pv, ofs, count) = match self.get_phys_block(idx + blk as u64, rem) {
				Some(v) => v,
				None => {
					log_warning!("VolumeHandle::write_blocks - Block id {} is invalid", idx + blk as u64);
					return Err( IoError::BadAddr )
					},
				};
			log_trace!("- PV{} {} + {}", pv, ofs, count);
			assert!(count <= rem);
			let bofs = blk as usize * self.block_size();
			let dst = &dst[bofs .. bofs + count * self.block_size()];
			S_PHYSICAL_VOLUMES.lock().get(&pv).unwrap().write(ofs, dst).await?;
			blk += count;
			rem -= count;
		}
		Ok( () )
	}
}

impl PhysicalVolumeInfo
{
	fn max_blocks_per_read(&self) -> usize {
		// 32 blocks per read op, = 0x4000 (16KB) for 512 byte sectors
		// TODO: Remove this?
		32
	}
	
	/// Read blocks from the device
	pub async fn read(&self, first: u64, dst: &mut [u8]) -> Result<usize,IoError>
	{
		log_trace!("PhysicalVolumeInfo::read(block {}, {} bytes)", first, dst.len());
		let block_size = self.dev.blocksize();
		let total_blocks = dst.len() / block_size;
		// Read up to 'block_step' blocks in each read call
		// - TODO: Request a read of as much as possible, and be told by the device how many were serviced
		{
			let mut buf = dst;
			let mut blk_id = first;
			while buf.len() > 0
			{
				assert!(buf.len() % block_size == 0);
				let prio = 0;
				let blocks = buf.len() / block_size;

				let real_count = match self.dev.read(prio, blk_id, blocks, buf).await
					{
					Ok(0) => todo!("Error when PV reports nothing read?"),
					Ok(v) => v,
					Err(e) => todo!("Error when PV fails to read: {:?}", e),
					};
				assert!(real_count <= blocks);
				blk_id += real_count as u64;

				// SAFE: Evil stuff to advance the buffer
				buf = unsafe { &mut *(&mut buf[real_count * block_size..] as *mut _) };
//				split_at_mut_inplace(&mut buf, real_count * block_size);
			}
		}

		log_trace!("PhysicalVolumeInfo::read(): total_blocks={}", total_blocks);
		Ok(total_blocks)
	}
	
	/// Write blocks from the device
	pub async fn write(&self, first: u64, dst: &[u8]) -> Result<usize,IoError>
	{
		log_trace!("PhysicalVolumeInfo::write(first={},{} bytes)", first, dst.len());
		let block_step = self.max_blocks_per_read();
		let block_size = self.dev.blocksize();
		// Read up to 'block_step' blocks in each read call
		{
			let iter_ids  = (first .. ).step_by(block_step);
			let iter_bufs = dst.chunks( block_step * block_size );
			for (blk_id,buf) in iter_ids.zip( iter_bufs )
			{
				let prio = 0;
				let blocks = buf.len() / block_size;
				
				// TODO: Async! (maybe return a composite read handle?)
				match self.dev.write(prio, blk_id, blocks, buf).await
				{
				Ok(real_count) => { assert!(real_count == blocks, "TODO: Handle incomplete writes"); },
				Err(e) => todo!("Error when PV fails to write: {:?}", e),
				}
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
	use crate::prelude::*;
	use crate::metadevs::storage;
	
	pub struct Mapper;
	
	pub static S_MAPPER: Mapper = Mapper;
	
	impl crate::metadevs::storage::Mapper for Mapper {
		fn name(&self) -> &str { "fallback" }
		fn handles_pv(&self, _pv: &dyn storage::PhysicalVolume) -> Result<usize,super::IoError> {
			// The fallback mapper never explicitly handles
			Ok(0)
		}
		fn enum_volumes(&self, pv: &dyn storage::PhysicalVolume, new_volume_cb: &mut dyn FnMut(String, u64, u64)) -> Result<(),super::IoError> {
			if let Some(cap) = pv.capacity() {
				new_volume_cb(format!("{}w", pv.name()), 0, cap );
			}
			Ok( () )
		}
	}
}

mod null_volume
{
	use crate::prelude::*;
	pub struct NullVolume;
	impl super::PhysicalVolume for NullVolume {

		fn name(&self) -> &str { "null" }
		fn blocksize(&self) -> usize { 512 }
		fn capacity(&self) -> Option<u64> { Some(0) }
		
		fn read<'a>(&'a self, _prio: u8, _blockidx: u64, _count: usize, _dst: &'a mut [u8]) -> super::AsyncIoResult<'a, usize> {
			Box::pin(async { Ok(0) })
		}
		fn write<'a>(&'a self, _prio: u8, _blockidx: u64, _count: usize, _src: &'a [u8]) -> super::AsyncIoResult<'a, usize> {
			Box::pin(async { Ok(0) })
		}
		fn wipe<'a>(&'a self, _blockidx: u64, _count: usize) -> super::AsyncIoResult<'a,()> {
			Box::pin(async { Ok(()) })
		}
	}
}

// vim: ft=rust
