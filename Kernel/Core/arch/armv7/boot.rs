//
//
//
use crate::lib::lazy_static::LazyStatic;
use crate::lib::fdt::FDTRoot;
use super::memory::addresses::IDENT_SIZE;

#[repr(C)]
#[derive(Debug)]
struct SymbolInfo {
	base: *const u8,
	count: usize,
	string_table: *const u8,
	strtab_len: usize,
}

extern "C" {
	static dt_phys_base: u32;
	static kernel_phys_start: u32;
	static symbol_info_phys: u32;
	static ram_first_free: u32;
	static mut kernel_exception_map: [u32; 1024];
	static v_kernel_end: crate::Extern;
}

enum BootInfo
{
	None,
	Basic(u32,u32),
	FDT(FDTRoot<'static>),
}

static S_FDT: LazyStatic<BootInfo> = LazyStatic::new();
static mut S_MEMMAP_DATA: [crate::memory::MemoryMapEnt; 16] = [crate::memory::MAP_PAD; 16];

pub fn get_fdt() -> Option<&'static FDTRoot<'static>> {
	match get_boot_info()
	{
	&BootInfo::FDT(ref fdt) => Some(fdt),
	_ => None,
	}
}

fn get_boot_info() -> &'static BootInfo {
	S_FDT.prep(|| BootInfo::new())
}

impl BootInfo
{
	fn new() -> BootInfo {
		// SAFE: Immutable
		let dt_phys = unsafe { dt_phys_base };
		log_debug!("dt_phys_base = {:#x}", dt_phys);
		if dt_phys == 0 {
			BootInfo::None
		}
		else {
			// SAFE: In practice, this is run in a single-thread. Any possible race would be benign
			unsafe {
				const FLAGS: u32 = 0x13;
				kernel_exception_map[1024-3] = dt_phys + FLAGS;
				kernel_exception_map[1024-2] = dt_phys + 0x1000 + FLAGS;
			}

			// SAFE: Address range checked
			unsafe {
				assert!(symbol_info_phys - kernel_phys_start < IDENT_SIZE as u32);
				let info: &'static SymbolInfo = &*((symbol_info_phys - kernel_phys_start + 0x80000000) as *const SymbolInfo);
				log_debug!("(symbol) info = {:?}", info);
				if !info.base.is_null() {
					let syms_addr = info.base as usize - kernel_phys_start as usize;
					let strs_addr = info.string_table as usize - kernel_phys_start as usize;
					assert!(syms_addr < IDENT_SIZE);
					assert!(strs_addr < IDENT_SIZE);
					let syms = ::core::slice::from_raw_parts( (syms_addr + 0x80000000) as *const crate::symbols::Elf32_Sym, info.count as usize);
					let strs = ::core::slice::from_raw_parts( (strs_addr + 0x80000000) as *const u8, info.strtab_len as usize);
					crate::symbols::set_symtab(syms, strs, 0);
				}
			}
			
			// SAFE: Memory is valid, and is immutable
			unsafe {
				BootInfo::FDT( FDTRoot::new_raw(0xFFFFD000 as *const u8) )
			}
		}
	}
}

pub fn get_video_mode() -> Option<crate::metadevs::video::bootvideo::VideoMode> {
	None
}

pub fn get_boot_string() -> &'static str {
	match get_boot_info()
	{
	&BootInfo::FDT(ref fdt) => fdt.get_props(&["","chosen","bootargs"]).next().map(|x| ::core::str::from_utf8(&x[..x.len()-1]).unwrap_or("") ).unwrap_or(""),
	_ => "",
	}
}

pub fn get_memory_map() -> &'static [crate::memory::MemoryMapEnt] {
	// TODO: Assert that this is only ever called once
	// SAFE: Assuming this function is called only once (which it is)
	let buf: &mut [_] = unsafe { &mut S_MEMMAP_DATA };
	let len = {
		let mut mapbuilder = crate::memory::MemoryMapBuilder::new(buf);
		match get_boot_info()
		{
		&BootInfo::None => {},
		&BootInfo::Basic(ram_base, ram_len) => {
			mapbuilder.append( ram_base as u64, ram_len as u64, crate::memory::MemoryState::Free, 0 );
			},
		&BootInfo::FDT(ref fdt) => {
			// FDT Present, need to locate all memory nodes
			fdt.dump_nodes();
			for prop in fdt.get_props_cb(|idx,leaf,name| match (idx,leaf)
				{
				(0,false) => name == "",
				(1,false) => name == "memory" || name.starts_with("memory@"),
				(2,true) => name == "reg",
				_ => false,
				})
			{
				use crate::lib::byteorder::{ReadBytesExt,BigEndian};
				let mut p = prop;
				let base = p.read_u64::<BigEndian>().unwrap();
				let size = p.read_u64::<BigEndian>().unwrap();
				log_debug!("base = {:#x}, size = {:#x}", base, size);
				mapbuilder.append( base, size, crate::memory::MemoryState::Free, 0 );
			}
			// SAFE: Immutable extern static
			mapbuilder.set_range( unsafe { dt_phys_base as u64 }, fdt.size() as u64, crate::memory::MemoryState::Used, 0 ).unwrap();
			},
		}

		// SAFE: Immutable statics
		unsafe { 
			if kernel_phys_start != 0 {
				// 2. Clobber out kernel, modules, and strings
				mapbuilder.set_range( kernel_phys_start as u64, &v_kernel_end as *const _ as u64 - 0x80000000, crate::memory::MemoryState::Used, 0 ).unwrap();
			}
			if ram_first_free != 0 {
				mapbuilder.set_range( kernel_phys_start as u64, (ram_first_free - kernel_phys_start) as u64, crate::memory::MemoryState::Used, 0 ).unwrap();
			}
		}
		
		mapbuilder.size()
		};
	&buf[..len]
}

