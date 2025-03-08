// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/arch/amd64/mptable.rs
//! Legacy Multi-Processor Tables

#[derive(Debug)]
pub struct MPTablePointer {
	_info: &'static MPInfo,
	table: &'static MPTable,
}

#[repr(C)]
pub struct ArrayStr<const N: usize>([u8; N]);
impl<const N: usize> ::core::fmt::Debug for ArrayStr<N> {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		f.write_str("b\"")?;
		for v in self.0.iter() {
			for b in v.escape_ascii() {
				::core::fmt::Write::write_char(f, b as char)?;
			}
		}
		f.write_str("\"")?;
		Ok( () )
	}
}

impl MPTablePointer
{
	pub fn locate_floating() -> Option<MPTablePointer>
	{
		// - EBDA/Last 1Kib (with 640KiB bsae memory)
		if let Some(rv) = Self::locate_floating_in(0x9F000, 0x1000) {
			return Some(rv);
		}
		// - Last KiB of 512KiB base mem
		if let Some(rv) = Self::locate_floating_in(0x7F000, 0x1000) {
			return Some(rv);
		}
		// The entire BIOS ROM (0xE_0000 to end of memory)
		if let Some(rv) = Self::locate_floating_in(0xE0000, 0x20000) {
			return Some(rv);
		}
		None
	}

	pub fn lapic_paddr(&self) -> u64 {
		self.table.local_apic_memory_map as u64
	}
	pub fn entries(&self) -> impl Iterator<Item=MPTableEntry>+'_ {
		self.table.entries()
	}
}
impl MPTablePointer
{
	fn locate_floating_in(start: usize, len: usize) -> Option<MPTablePointer> {
		assert!(start % 4 == 0);
		assert!(len % 4 == 0);
		// SAFE: Accessing valid memory, and shouldn't be aliased
		unsafe {
			let search_base = crate::arch::memory::virt::fixed_alloc(start as u64, (start+0x1000-1)/0x1000).unwrap();
			let search_slice = core::slice::from_raw_parts(search_base as *const MPInfo, len / 16);
			for v in search_slice {
				// ('_'|('M'<<8)|('P'<<16)|('_'<<24))
				if v.signature == 0x5f_50_4d_5f {
					if v.calc_checksum() == 0 {
						let mpt = v.get_mptable();
						log_debug!("MPTable: {:#x?}", mpt);
						for e in mpt.entries() {
							log_debug!("MPTable Ent: {:x?}", e);
						}
						return Some(MPTablePointer { _info: v, table: mpt });
					}
				}
			}
		}
		None
	}
}

#[repr(C)]
#[derive(Debug)]
struct MPInfo
{
	signature: u32,	// '_MP_'
	mpconfig: u32,
	length: u8,
	version: u8,
	checksum: u8,
	features: [u8; 5],
}

#[repr(C)]
#[derive(Debug)]
struct MPTable
{
	signature: u32,
	base_table_length: u16,
	specification_revision: u8,
	checksum: u8,

	oem_id: ArrayStr<8>,
	product_id: ArrayStr<12>,

	oem_table_ptr: u32,
	oem_table_size: u16,

	entry_count: u16,

	/// Address used to access the local APIC
	local_apic_memory_map: u32,

	extended_table_len: u16,
	extended_table_checksum: u8,
	_reserved: u8,

	//entries: [MPTableEnt],
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum MPTableEntry<'a> {
	Unknown(u8),
	Proc(&'a MPTableEntry_Proc),
	Bus(&'a MPTableEntry_Bus),
	IoApic(&'a MPTableEntry_IoApic),
	/// I/O Interrupt Assignment
	IoIntAssign(&'a MPTableEntry_IoIntAssign),
	LocalIntAssign(&'a MPTableEntry_LocalIntAssign),
}
#[repr(C)]
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct MPTableEntry_Proc {
	pub tag: u8,	// 0x00
	pub apic_id: u8,
	pub apic_ver: u8,
	pub cpu_flags: u8,
	pub cpu_signature: u32,
	pub feature_flags: u32,
	_reserved: [u32; 2],
}
#[repr(C)]
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct MPTableEntry_Bus {
	pub tag: u8,	// 0x01
	pub id: u8,
	pub type_string: ArrayStr<6>,
}
#[repr(C)]
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct MPTableEntry_IoApic {
	pub tag: u8,	// 0x02
	pub id: u8,
	pub version: u8,
	pub flags: u8,
	pub addr: u32,
}
#[repr(C)]
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct MPTableEntry_IoIntAssign {
	pub tag: u8,	// 0x01
	pub int_type: u8,
	/// 0,1: Polarity, 2,3: Trigger Mode
	pub flags: u16,
	pub source_bus_id: u8,
	pub source_bus_irq: u8,
	pub dest_ioapic_id: u8,
	pub dest_ioapic_irq: u8,
}
#[repr(C)]
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct MPTableEntry_LocalIntAssign {
	pub tag: u8,	// 0x01
	pub int_type: u8,
	/// 0,1: Polarity, 2,3: Trigger Mode
	pub flags: u16,
	pub source_bus_id: u8,
	pub source_bus_irq: u8,
	pub dest_lapic_id: u8,
	pub dest_lapic_irq: u8,
}

impl MPInfo
{
	fn calc_checksum(&self) -> u8 {
		// SAFE: `Self` is POD
		let byte_slice = unsafe { ::core::slice::from_raw_parts(self as *const _ as *const u8, ::core::mem::size_of::<Self>()) };
		byte_slice.iter().sum()
	}

	fn get_mptable(&self) -> &'static MPTable {
		// SAFE: Read-only access of read-only memory
		unsafe {
			if let Some(ptr) = crate::arch::memory::virt::fixed_alloc(self.mpconfig as u64, (::core::mem::size_of::<MPTable>()+0x1000-1)/0x1000) {
				&*(ptr as *const MPTable)
			}
			else {
				todo!("MPTable isn't in identity range: {:#x}", self.mpconfig);
			}
		}
	}
}

impl MPTable
{
	fn entries(&self) -> impl Iterator<Item=MPTableEntry>+'_ {
		struct Iter<'a> {
			pd: ::core::marker::PhantomData<&'a MPTable>,
			cur: *const [u8; 4],
			count: usize,
		}
		impl<'a> Iter<'a> {
			unsafe fn get<T: 'a>(&self, cb: impl FnOnce(&'a T)->MPTableEntry<'a>) -> (MPTableEntry<'a>, usize,) {
				(cb(&*(self.cur as *const _)), ::core::mem::size_of::<T>(),)
			}
		}
		impl<'a> Iterator for Iter<'a> {
			type Item = MPTableEntry<'a>;
			fn next(&mut self) -> Option<MPTableEntry<'a>> {
				if self.count == 0 {
					return None;
				}
				// SAFE: Pointer should be valid.. assuming the mptable was
				unsafe {
					let (rv, size) = match (*self.cur)[0]
						{
						0x00 => self.get(|r| MPTableEntry::Proc(r)),
						0x01 => self.get(|r| MPTableEntry::Bus(r)),
						0x02 => self.get(|r| MPTableEntry::IoApic(r)),
						0x03 => self.get(|r| MPTableEntry::IoIntAssign(r)),
						0x04 => self.get(|r| MPTableEntry::LocalIntAssign(r)),
						v => {
							self.count = 0;
							return Some(MPTableEntry::Unknown(v))
							},
						};
					self.cur = self.cur.offset(size as isize / 4);
					self.count -= 1;
					Some(rv)
				}
			}
		}
		Iter {
			pd: ::core::marker::PhantomData,
			// SAFE: Pointer offset is in-bounds
			cur: unsafe { (self as *const Self).offset(1) as *const [u8; 4] },
			count: self.entry_count as usize,
		}
	}
}
