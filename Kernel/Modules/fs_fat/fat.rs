//!
//!
use kernel::metadevs::storage;
use kernel::lib::byteorder::{ReadBytesExt,LittleEndian};
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
	pub fn alloc_cluster_chained(&self, prev_cluster: u32) -> Result< Option<u32>, storage::IoError > {
		let Some(cluster_idx) = self.alloc_cluster_unchained(prev_cluster)? else { return Ok(None); };
		// Update the previous cluster's chain from EOC to this
		self.set_fat_entry(prev_cluster, FatEntry::EndOfChain, FatEntry::Chain(cluster_idx))?;
		Ok( Some(cluster_idx) )
	}

	/// Allocate a new cluster as the start of a new chain (use the previous cluster to maybe reduce fragmentation)
	pub fn alloc_cluster_unchained(&self, prev_cluster: u32) -> Result< Option<u32>, storage::IoError > {
		// Search for an unallocated cluster in the FAT, starting from `prev_cluster`
		// - May need to use a pre-allocated bitmap to speed up allocation?
		// - Or just iterate the FAT
		let cps = match self.ty
			{
			Size::Fat12 => (self.vh.block_size() / 3 * 2) as u32,	// 2 per 3 bytes
			Size::Fat16 => (self.vh.block_size() / 2) as u32,
			Size::Fat32 => (self.vh.block_size() / 4) as u32,
			};
		let aligned = if (prev_cluster + 1) % cps != 0 {
				let idx_in_sector = (prev_cluster + 1) % cps;
				let base = prev_cluster + 1 - idx_in_sector;
				if let Some(rv) = self.find_and_alloc_cluster_in_sector(base, idx_in_sector, cps)? {
					return Ok(Some(rv));
				}
				base + cps
			} else {
				prev_cluster + 1
			};
		// Iterate until the end of the list
		for base in (aligned .. self.cluster_count as u32).step_by(cps as usize)
		{
			if let Some(rv) = self.find_and_alloc_cluster_in_sector(base, 0, cps)? {
				return Ok(Some(rv));
			}
		}

		// Loop back around until the starting point.
		if aligned != 0
		{
			for base in (0 .. aligned - cps).step_by(cps as usize)
			{
				if let Some(rv) = self.find_and_alloc_cluster_in_sector(base, 0, cps)? {
					return Ok(Some(rv));
				}
			}
		}
		if aligned != prev_cluster + 1
		{
			assert!(aligned > 0);
			let base = aligned - cps;
			if let Some(rv) = self.find_and_alloc_cluster_in_sector(aligned - cps, 0, prev_cluster + 1 - base)? {
				return Ok(Some(rv));
			}
		}

		Ok(None)
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

	// TODO: A deallocation that chains?
}

#[derive(Copy,Clone,Debug)]
enum FatEntry {
	Unallocated,
	EndOfChain,
	Chain(u32),
}
impl FatEntry {
	fn from_fat12_outer(val: u32, is_second: bool) -> Self {
		Self::from_fat12(if is_second { (val >> 12) as u16 } else { (val & 0xFFF) as u16 })
	}
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
	fn to_fat12_outer(self, prev: u32, is_second: bool) -> u32 {
		if is_second {
			(prev & 0xFFF) | (self.to_fat12() as u32) << 12
		}
		else {
			(prev & 0xFFF_000) | (self.to_fat12() as u32) << 0
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
	fn get_fat_addr(&self, cluster: u32) -> (u64, usize, usize, u32) {
		// - Determine what sector contains the requested FAT entry
		let bs = self.vh.block_size();
		let cps;
		let (ofs, ent_len) = match self.ty
			{
			Size::Fat12 => {
				cps = bs as u32 / 3 * 2;	// 2 per 3 bytes
				( (cluster % cps) / 2 * 3, 3 )
				},
			Size::Fat16 => {
				cps = bs as u32 / 2;
				( (cluster % cps) * 2, 2, )
				},
			Size::Fat32 => {
				cps = bs as u32 / 4;
				( (cluster % cps) * 4, 4, )
				},
			};
		let sector_idx = self.first_fat_sector as u64 + (cluster / cps) as u64;
		(sector_idx, ofs as usize, ent_len, cps)
	}

	/// Read a FAT entry
	fn get_fat_entry(&self, cluster: u32) -> Result<FatEntry, storage::IoError> {
		let (sector_idx, ofs, ent_len, _cps) = self.get_fat_addr(cluster);

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
		let (sector_idx, ofs, ent_len, _cps) = self.get_fat_addr(cluster);

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
				write_u24_le(buf, newval);
				},
			// Simple read+check and write for FAT16/FAT32
			Size::Fat16 => {
				let val = (&buf[..]).read_u16::<LittleEndian>().unwrap();
				if val != exp_prev.to_fat16() {
					return false;
				}
				write_u16_le(buf, new.to_fat16());
				},
			Size::Fat32 => {
				let val = (&buf[..]).read_u32::<LittleEndian>().unwrap();
				if val != exp_prev.to_fat32() {
					return false;
				}
				write_u32_le(buf, new.to_fat32());
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

	/// Find an unallocated cluster in this FAT sector, and mark it as allocated (using EOC)
	/// * `base`: The index of the first cluster in this FAT sector
	/// * `start`: Sector-internal index of the first cluster to consider
	/// * `end`: Sector-internal index of the past-end cluster to consider
	fn find_and_alloc_cluster_in_sector(&self, base: u32, start: u32, end: u32) -> Result<Option<u32>, storage::IoError>
	{
		assert!(base < self.cluster_count as u32);
		let (sector_idx, _ofs, _ent_size, cps) = self.get_fat_addr(base);
		assert!(base % cps == 0);
		// Clamp the end to CPS (should be already), and to the last cluster in the volume (may not be)
		let end = end.min(cps).min(self.cluster_count as u32 - base);
		::kernel::futures::block_on( self.vh.edit(sector_idx, 1, |data| {
			match self.ty
			{
			Size::Fat12 => {
				for sub_idx in start .. end {
					let buf = &mut data[sub_idx as usize / 2 * 3..][..3];
					let val = {&buf[..]}.read_uint::<LittleEndian>(3).unwrap() as u32;
					if let FatEntry::Unallocated = FatEntry::from_fat12_outer(val, sub_idx % 2 == 1) {
						write_u24_le(buf, FatEntry::EndOfChain.to_fat12_outer(val, sub_idx % 2 == 1));
						return Some(base + sub_idx);
					}
				}
				},
			Size::Fat16 => {
				for sub_idx in start .. end {
					let buf = &mut data[sub_idx as usize / 2..][..2];
					let val = {&buf[..]}.read_u16::<LittleEndian>().unwrap();
					if let FatEntry::Unallocated = FatEntry::from_fat16(val) {
						write_u16_le(buf, FatEntry::EndOfChain.to_fat16());
						return Some(base + sub_idx);
					}
				}
				},
			Size::Fat32 => {
				for sub_idx in start .. end {
					let buf = &mut data[sub_idx as usize / 4..][..2];
					let val = {&buf[..]}.read_u32::<LittleEndian>().unwrap();
					if let FatEntry::Unallocated = FatEntry::from_fat32(val) {
						write_u32_le(buf, FatEntry::EndOfChain.to_fat32());
						return Some(base + sub_idx);
					}
				}
				},
			}
			None
			}) )
	}
}

fn write_u24_le(dst: &mut [u8], val: u32) {
	dst[0] = (val >>  0) as u8;
	dst[1] = (val >>  8) as u8;
	dst[2] = (val >> 16) as u8;
}
fn write_u16_le(dst: &mut [u8], val: u16) {
	dst.copy_from_slice(&val.to_le_bytes());
}
fn write_u32_le(dst: &mut [u8], val: u32) {
	dst.copy_from_slice(&val.to_le_bytes());
}
