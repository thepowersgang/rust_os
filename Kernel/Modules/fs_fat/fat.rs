//!
//!
use kernel::metadevs::storage;
use super::Size;

// End-of-chain marker values
const FAT12_EOC: u16 = 0x0FFF;
const FAT16_EOC: u16 = 0xFFFF;
const FAT32_EOC: u32 = 0x00FFFFFF;	// FAT32 is actually FAT24...

/// FAT management methods
impl super::FilesystemInner
{
	/// Obtain the next cluster in a chain
	pub fn get_next_cluster(&self, cluster: u32) -> Result< Option<u32>, storage::IoError > {
		match self.get_fat_entry(cluster)?
		{
		FatEntry::Unallocated => Err(storage::IoError::Unknown("FAT: Zero FAT entry")),
		FatEntry::EndOfChain => Ok(None),
		FatEntry::Chain(val) => Ok(Some(val)),
		}
	}

	/// Allocate a new cluster, and append it to the FAT chain
	pub fn alloc_cluster_chained(&self, prev_cluster: u32) -> Result< u32, storage::IoError > {
		let cluster_idx = self.alloc_cluster_unchained(prev_cluster)?;
		// Update the previous cluster's chain from EOC to this
		self.set_fat_entry(prev_cluster, FatEntry::EndOfChain, FatEntry::Chain(cluster_idx))?;
		Ok( cluster_idx )
	}
	/// Allocate a new cluster as the start of a new chain (use the previous cluster to maybe reduce fragmentation)
	pub fn alloc_cluster_unchained(&self, prev_cluster: u32) -> Result< u32, storage::IoError > {
		// Search for an unallocated cluster in the FAT, starting from `prev_cluster`
		// - May need to use a pre-allocated bitmap to speed up allocation?
		todo!("alloc_cluster_unchained(prev={:#x})", prev_cluster)
	}
	/// Deallocate a cluster (at the end of a chain)
	pub fn release_cluster(&self, cluster_idx: u32, prev: Option<u32>) -> Result<(), storage::IoError> {
		if let Some(prev) = prev {
			// Set the entry to EOC, must have been `cluster_idx`
			self.set_fat_entry(prev, FatEntry::Chain(cluster_idx), FatEntry::EndOfChain)?;
		}
		// Set this cluster to 0 (must have been EOC)
		self.set_fat_entry(cluster_idx, FatEntry::EndOfChain, FatEntry::Unallocated)?;
		Ok( () )
	}
}

#[derive(Copy,Clone,Debug)]
enum FatEntry {
	Unallocated,
	EndOfChain,
	Chain(u32),
}
impl FatEntry {
	fn from_fat12(val: u16) -> Self {
		match val {
		0 => FatEntry::Unallocated,
		FAT12_EOC => FatEntry::EndOfChain,
		val => FatEntry::Chain(val as u32),
		}
	}
	fn from_fat16(val: u16) -> Self {
		match val {
		0 => FatEntry::Unallocated,
		FAT16_EOC => FatEntry::EndOfChain,
		val => FatEntry::Chain(val as u32),
		}
	}
	fn from_fat32(val: u32) -> Self {
		match val {
		0 => FatEntry::Unallocated,
		FAT32_EOC => FatEntry::EndOfChain,
		val => FatEntry::Chain(val),
		}
	}

	fn to_fat12(self) -> u16 {
		match self {
		FatEntry::Unallocated => 0,
		FatEntry::EndOfChain => FAT12_EOC,
		FatEntry::Chain(val) => val as u16,
		}
	}
	fn to_fat16(self) -> u16 {
		match self {
		FatEntry::Unallocated => 0,
		FatEntry::EndOfChain => FAT16_EOC,
		FatEntry::Chain(val) => val as u16,
		}
	}
	fn to_fat32(self) -> u32 {
		match self {
		FatEntry::Unallocated => 0,
		FatEntry::EndOfChain => FAT32_EOC,
		FatEntry::Chain(val) => val,
		}
	}
}

impl super::FilesystemInner
{
	fn get_fat_addr(&self, cluster: u32) -> (u64, usize, usize) {
		// - Determine what sector contains the requested FAT entry
		let bs = self.vh.block_size();
		let (fat_sector, ofs, ent_len) = match self.ty
			{
			Size::Fat12 => {
				let cps = bs / 3 * 2;	// 2 per 3 bytes
				(cluster as usize / cps, (cluster as usize % cps) / 2 * 3, 3 )
				},
			Size::Fat16 => {
				let cps = bs / 2;
				(cluster as usize / cps, (cluster as usize % cps) * 2, 2)
				},
			Size::Fat32 => {
				let cps = bs / 4;
				(cluster as usize / cps, (cluster as usize % cps) * 4, 4)
				},
			};
		let sector_idx = (self.first_fat_sector + fat_sector) as u64;
		(sector_idx, ofs, ent_len)
	}

	/// Read a FAT entry
	fn get_fat_entry(&self, cluster: u32) -> Result<FatEntry, storage::IoError> {
		use kernel::lib::byteorder::{ReadBytesExt,LittleEndian};
		let (sector_idx, ofs, ent_len) = self.get_fat_addr(cluster);

		// - Read entry from the FAT
		let mut buf = [0; 4];
		::kernel::futures::block_on( self.vh.read_inner(sector_idx, ofs, &mut buf[..ent_len]) )?;
		let mut buf = &buf[..ent_len];

		// - Extract the entry
		Ok(match self.ty
		{
		// FAT12 has special handling because it packs 2 entries into 24 bytes
		Size::Fat12 => FatEntry::from_fat12({
				let v24 = buf.read_uint::<LittleEndian>(3).unwrap();
				if cluster % 2 == 0 { v24 & 0xFFF } else { v24 >> 12 }
				} as u16),
		Size::Fat16 => FatEntry::from_fat16(buf.read_u16::<LittleEndian>().unwrap()),
		Size::Fat32 => FatEntry::from_fat32(buf.read_u32::<LittleEndian>().unwrap()),
		})
	}
	/// Update a FAT entry, checking the previous value
	fn set_fat_entry(&self, cluster: u32, exp_prev: FatEntry, new: FatEntry) -> Result< (), storage::IoError > {
		use kernel::lib::byteorder::{ReadBytesExt,LittleEndian};
		let (sector_idx, ofs, ent_len) = self.get_fat_addr(cluster);

		// Use `block_cache`'s read/write locks
		let changed = ::kernel::futures::block_on(self.vh.edit(sector_idx, 1, |buf| {
			let buf = &mut buf[ofs..][..ent_len];
			match self.ty
			{
			// FAT12 has special handling because it packs 2 entries into 24 bytes
			Size::Fat12 => {
				let val = (&buf[..]).read_uint::<LittleEndian>(3).unwrap() as u32;
				let newval = if cluster % 2 == 0 {
						let cur = (val & 0xFFF) as u16;
						if cur != exp_prev.to_fat12() {
							log_error!("FAT Check failure: {:#x} expected {:?} got {:?}",
								cluster, exp_prev, FatEntry::from_fat12(cur));
							return false;
						}
						(val & 0xFFF_000) | (new.to_fat12() as u32) << 0
					}
					else {
						let cur = (val >> 12) as u16;
						if cur != exp_prev.to_fat12() {
							log_error!("FAT Check failure: {:#x} expected {:?} got {:?}",
								cluster, exp_prev, FatEntry::from_fat12(cur));
							return false;
						}
						(val & 0x000_FFF) | (new.to_fat12() as u32) << 12
					};
				buf[0] = (newval >>  0) as u8;
				buf[1] = (newval >>  8) as u8;
				buf[2] = (newval >> 16) as u8;
				},
			// Simple read+check and write for FAT16/FAT32
			Size::Fat16 => {
				let val = (&buf[..]).read_u16::<LittleEndian>().unwrap();
				if val != exp_prev.to_fat16() {
					return false;
				}
				let newval = new.to_fat16();
				buf.copy_from_slice(&newval.to_le_bytes());
				},
			Size::Fat32 => {
				let val = (&buf[..]).read_u32::<LittleEndian>().unwrap();
				if val != exp_prev.to_fat32() {
					return false;
				}
				let newval = new.to_fat32();
				(&mut buf[..]).copy_from_slice(&newval.to_le_bytes());
				},
			}
			true
			}))?;

		if !changed {
			Err(storage::IoError::Unknown("FAT: Internal assertion failure"))
		}
		else {
			Ok( () )
		}
	}
}
