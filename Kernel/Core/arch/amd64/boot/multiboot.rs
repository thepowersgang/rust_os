///! Multiboot (version 1) boot information
use super::super::memory::addresses::{IDENT_START, IDENT_END};
use crate::metadevs::video::bootvideo::{VideoMode,VideoFormat};
use crate::symbols::Elf32_Sym;
use super::SymbolInfo;
use super::super::v_kernel_end;
use super::valid_c_str_to_slice;

pub const MAGIC: u32 = 0x2BADB002;

/// A parsed version of multiboot v1 data
pub struct MultibootParsed
{
	pub cmdline: &'static str,
	pub vidmode: Option<VideoMode>,
	pub memmap: &'static [crate::memory::MemoryMapEnt],
	pub symbol_info: SymbolInfo,
	pub modules: &'static [super::ModuleInfo],
}


#[repr(C)]
#[allow(unused)]
struct MultibootInfo
{
	flags: u32,
	// flags[0]
	lomem: u32, himem: u32,
	// flags[1]
	bootdev: u32,
	// flags[2]
	cmdline: u32,
	// flags[3]
	module_count: u32, module_first: u32,
	// flags[4] or flags[5]
	syminfo: [u32; 4],
	// flags[6]
	memmap_len: u32,
	memmap_ptr: u32,
	// flags[7]
	drives_length: u32,
	drives_addr: u32,
	// flags[8]
	configtable_ptr: u32,	// result of BIOS 'GET CONFIGURATION'
	// flags[9]
	boot_loader_name: u32,	// C string, booting loader
	// flags[10]
	apm_table_ptr: u32,
	// flags[11]
	vbe_control_info: u32,
	vbe_mode_info: u32,
	vbe_mode: u32,
	vbe_interface_seg: u32,
	vbe_interface_off: u32,
	vbe_interface_len: u32,
}

#[repr(C)]
#[allow(unused)]
#[derive(Debug)]
struct VbeModeInfo
{
	attributes: u16,
	window_attrs: [u8; 2],
	granularity: u16,
	window_size: u16,
	window_segments: [u16; 2],
	win_pos_fcn_fptr: [u16; 2],	// Pointer to INT 10h, AX=4F05h
	
	pitch: u16,
	x_res: u16, y_res: u16,
	char_w: u8, char_h: u8,
	n_planes: u8,
	bpp: u8,
	n_banks: u8,
	memory_model: u8,
	bank_size: u8,
	n_pages: u8,
	_resvd: u8,	// reserved
	
	// VBE 1.2+
	red_mask: u8,	red_position: u8,
	green_mask: u8, green_position: u8,
	blue_mask: u8,  blue_position: u8,
	rsv_mask: u8,   rsv_position: u8,
	directcolor_attributes: u8,

	// VBE v2.0+
	physbase: u32,
	offscreen_ptr: u32,	// Start of offscreen memory
	offscreen_size_kb: u16,	// Size of offscreen memory
	
	// -- VBE v3.0
	lfb_pitch: u16,
	image_count_banked: u8,
	image_count_lfb: u8,
}

#[repr(C)]
struct ModuleInfo
{
	mod_start: u32,
	mod_end: u32,
	string_addr: u32,
	_reserved: u32,
}

impl MultibootParsed
{
	pub unsafe fn from_ptr(
		info_ptr: *const crate::Void,
		mmap_buf: &'static mut [crate::memory::MemoryMapEnt],
		mod_buf: &'static mut [super::ModuleInfo],
	) -> Option<MultibootParsed>
	{
		let info = &*(info_ptr as *const MultibootInfo);
		//if info.flags & !0xFFF != 0 {
		//	log_error!("Multiboot header malformed (reserved flag bits set {:#x})", info.flags);
		//	return None;
		//}
		let loader_name = if (info.flags & 1 << 9) != 0 {
				let loader_ptr = (info.boot_loader_name as usize + IDENT_START) as *const i8;
				log_debug!("loader_ptr = {:?}", loader_ptr);
				// SAFE: Loader string is valid for 'static
				unsafe { valid_c_str_to_slice(loader_ptr).unwrap_or("-INVALID-") }
			}
			else {
				"-UNKNOWN-"
			};
		
		
		log_notice!("Loading multiboot from loader '{}' (flags = {:#x})", loader_name, info.flags);
		let mut ret = MultibootParsed {
				cmdline: MultibootParsed::decode_cmdline(info),
				vidmode: MultibootParsed::decode_video_mode(info),
				symbol_info: MultibootParsed::decode_sym_info(info),
				modules: MultibootParsed::decode_modules(info, mod_buf),
				memmap: &[],
			};
		ret.memmap = ret.fill_memmap(info, mmap_buf);
		Some( ret )
	}

	fn decode_sym_info(info: &MultibootInfo) -> SymbolInfo
	{
		// Symbol information
		match (info.flags >> 4) & 3
		{
		0 => SymbolInfo::None,	// No symbol information
		1 => {
			// a.out symbol table
			let [tabsize, strsize, addr, _resvd] = info.syminfo;
			log_debug!("Symbols a.out - tabsize={}, strsize={}, addr={:#x}", tabsize, strsize, addr);
			SymbolInfo::None
			},
		2 => {
			use crate::memory::PAddr;

			let [num, size, addr, shndx] = info.syminfo;
			log_debug!("Symbols ELF - num={}, size={}, addr={:#x}, shndx={}", num, size, addr, shndx);
			#[allow(non_camel_case_types)]
			type Elf32_Word = u32;
			#[allow(non_camel_case_types)]
			type Elf32_Addr = u32;
			#[allow(non_camel_case_types)]
			type Elf32_Off = u32;
			struct ShEnt {
				sh_name: Elf32_Word,
				sh_type: Elf32_Word,
				sh_flags: Elf32_Word,
				sh_addr: Elf32_Addr,
				sh_offset: Elf32_Off,
				sh_size: Elf32_Word,
				sh_link: Elf32_Word,
				sh_info: Elf32_Word,
				sh_addralign: Elf32_Word,
				sh_entsize: Elf32_Word,
			}
			impl_fmt! {
				Debug(self, f) for ShEnt {
					write!(f, "ShEnt {{ name: {}, type: {}, flags: {:x}h, addr: {:#x}, offset: {:#x}, size: {:#x}, link: {}, info: {:#x}, addralign: {}, entsize: {} }}",
						self.sh_name, self.sh_type, self.sh_flags, self.sh_addr, self.sh_offset, self.sh_size, self.sh_link,
						self.sh_info, self.sh_addralign, self.sh_entsize
						)
				}
			}
			assert_eq!( ::core::mem::size_of::<ShEnt>(), size as usize );
			// SAFE: No aliasing
			let shtab: &'static [ShEnt] = match unsafe { crate::memory::virt::map_static_slice(addr as PAddr, num as usize) }
				{
				Ok(v) => v,
				Err(_) => &[],
				};
			for shent in shtab
			{
				//log_trace!("shent = {:?}", shent);
				if shent.sh_type == 2
				{
					let count = shent.sh_size as usize / ::core::mem::size_of::<Elf32_Sym>();
					// SAFE: Un-aliased
					let ents: &'static [_] = match unsafe { crate::memory::virt::map_static_slice(shent.sh_addr as PAddr, count) }
						{
						Ok(v) => v,
						Err(_) => break,
						};
					let strtab_ent = &shtab[shent.sh_link as usize];
					// SAFE: Un-aliased
					let strtab: &'static [u8] = match unsafe { crate::memory::virt::map_static_slice(strtab_ent.sh_addr as PAddr, strtab_ent.sh_size as usize) }
						{
						Ok(v) => v,
						Err(_) => break,
						};
					//log_debug!("ents = {:p}+{}, strtab={:?}", ents.as_ptr(), count, ::core::str::from_utf8_unchecked(strtab) );
					// SAFE: Called in single-threaded context
					unsafe { crate::symbols::set_symtab(ents, strtab, 0xFFFFFFFF_00000000); }
					return SymbolInfo::Elf32(ents, strtab);
				}
			}
			SymbolInfo::None
			},
		_ => {
			log_error!("Multiboot header malformed (both symbol table bits set)");
			SymbolInfo::None
			},
		}
	}
	
	fn decode_cmdline(info: &MultibootInfo) -> &'static str
	{
		if (info.flags & 1 << 2) == 0 {
			return "";
		}
		
		let cmdline_paddr = info.cmdline as usize;
		if cmdline_paddr + IDENT_START >= IDENT_END {
			return "";
		}
		
		let charptr = (cmdline_paddr + IDENT_START) as *const i8;
		// SAFE: Boot string is valid for 'static
		unsafe { valid_c_str_to_slice(charptr).unwrap_or("-INVALID-") }
	}
	
	fn decode_video_mode(info: &MultibootInfo) -> Option<VideoMode>
	{
		if (info.flags & 1 << 11) == 0 {
			log_notice!("get_video_mode - Video mode information not present");
			return None;
		}
		
		// SAFE: VBE info pointer should be valid for all of this call.
		let info = unsafe {
			let vbeinfo_vaddr = info.vbe_mode_info as usize + IDENT_START;
			if vbeinfo_vaddr + ::core::mem::size_of::<VbeModeInfo>() > IDENT_END {
				return None;
			}
			&*(vbeinfo_vaddr as *const VbeModeInfo)
			};
		
		log_trace!("MultibootInfo::_vidmode: info = {:?}", info);
		let pos_tuple = (info.red_position,info.green_position,info.blue_position);
		let size_tuple = (info.red_mask, info.green_mask, info.blue_mask);
		let fmt = match info.bpp
			{
			32 => match (pos_tuple, size_tuple)
				{
				((16,8,0), (8,8,8)) => VideoFormat::X8R8G8B8,	// 8:8:8:8 32BPP
				_ => todo!("MultibootInfo::_vidmode 32 pos={:?},size={:?}", pos_tuple, size_tuple),
				},
			24 => todo!("MultibootInfo::_vidmode: 24bpp"),
			16 => match (pos_tuple, size_tuple)
				{
				((11,5,0), (5,6,5)) => VideoFormat::R5G6B5,	// 5:6:5 16BPP
				//((10,5,0), (5,5,5)) => VideoFormat::X1R5G5B5,	// 5:5:5 15BPP
				_ => todo!("MultibootInfo::_vidmode: pos={:?},size={:?}", pos_tuple, size_tuple),
				},
			_ => {
				return None;
				},
			};
		
		Some( VideoMode {
			width: info.x_res,
			height: info.y_res,
			fmt: fmt,
			pitch: info.pitch as usize,
			base: info.physbase as crate::arch::memory::PAddr,
			})
	}

	fn fill_memmap<'a>(&self, info: &MultibootInfo, buf: &'a mut [crate::memory::MemoryMapEnt]) -> &'a [crate::memory::MemoryMapEnt]
	{
		let size = {
			let mut mapbuilder = crate::memory::MemoryMapBuilder::new(buf);
			// 1. Get raw map
			if false && (info.flags & 1 << 6) != 0 {
				// Full memory map
				panic!("TODO: Full memory map");
			}
			else if (info.flags & 1 << 0) != 0 {
				// Dumb memory map
				log_debug!("info = {{..., .lomem={}, .himem={} }}", info.lomem, info.himem);
				// - Low memory (before VGA BIOS)
				assert!(info.lomem >= 625);
				assert!(info.lomem <= 640);
				let top_lowmem = info.lomem as u64 * 1024;
				mapbuilder.append( 0x1000, top_lowmem - 0x1000, crate::memory::MemoryState::Free, 0 );
				// - High memory (above 1MiB)
				mapbuilder.append( 0x100000, info.himem as u64 * 1024, crate::memory::MemoryState::Free, 0 );
			}
			else {
				// No memory map
				panic!("TODO: Assumption memory map");
			}
			mapbuilder.sort();
			// TODO: Fix if not valid
			assert!( mapbuilder.validate() );
			
			// 2. Clobber out boot info
			// - Kernel
			// SAFE: Just taking the address
			let kernel_start = unsafe { &v_kernel_end as *const _ as u64 - IDENT_START as u64 };
			mapbuilder.set_range( 0x100000, kernel_start - 0x10000,
				crate::memory::MemoryState::Used, 0 ).ok().unwrap();
			// - Command line string
			mapbuilder.set_range( self.cmdline.as_ptr() as u64 - IDENT_START as u64, self.cmdline.len() as u64,
			crate::memory::MemoryState::Used, 0 ).ok().unwrap();
			// - Symbol information
			match self.symbol_info
			{
			SymbolInfo::None => {},
			SymbolInfo::Elf32(sym, str) => {
				mapbuilder.set_range( crate::memory::virt::get_phys(sym.as_ptr()), (sym.len() * ::core::mem::size_of::<Elf32_Sym>()) as u64,
					crate::memory::MemoryState::Used, 0).ok().unwrap();
				mapbuilder.set_range( crate::memory::virt::get_phys(str.as_ptr()), str.len() as u64,
					crate::memory::MemoryState::Used, 0).ok().unwrap();
				},
			}
			for m in self.modules
			{
				mapbuilder.set_range(m.base, m.length as u64, crate::memory::MemoryState::Used, 0)
					.ok().unwrap();
			}
			
			mapbuilder.size()
			};
		
		// 3. Return final result
		&buf[0 .. size]
	}


	/// UNSAFE: Caller is responsible for the contents of `info`
	unsafe fn decode_modules<'a>(info: &MultibootInfo, mod_buf: &'a mut [super::ModuleInfo]) -> &'a [super::ModuleInfo]
	{
		let mod_info = crate::memory::virt::map_static_slice::<ModuleInfo>(
			info.module_first as u64,
			info.module_count as usize
		).unwrap_or(&[]);

		for (d,s) in Iterator::zip(mod_buf.iter_mut(), mod_info.iter()) {
			*d = super::ModuleInfo {
				base: s.mod_start as u64,
				length: (s.mod_end - s.mod_start) as usize,
				name: valid_c_str_to_slice(
					crate::memory::virt::map_static(s.string_addr as u64)
						.unwrap_or(&0)
					).unwrap_or("")
				};
		}

		let rv = usize::min(mod_buf.len(), mod_info.len());
		&mod_buf[..rv]
	}
}

pub unsafe fn get_video(info_ptr: *const crate::Void) -> Option<VideoMode> {
	let info = &*(info_ptr as *const MultibootInfo);
	MultibootParsed::decode_video_mode(info)
}