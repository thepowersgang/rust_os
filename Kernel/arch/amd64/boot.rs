// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/boot.rs
// - Boot information
use _common::*;
use super::memory::addresses::{IDENT_START, IDENT_END};

#[repr(C)]
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
	syminfo: [u32,..4],
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
struct VbeModeInfo
{
	attributes: u16,
	window_attrs: [u8,..2],
	granuality: u16,
	window_size: u16,
	window_segments: [u16, ..2],
	win_pos_fcn_fptr: [u16, ..2],	// Pointer to INT 10h, AX=4F05h
	
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
	
}

struct MultibootParsed
{
	cmdline: &'static str,
	vidmode: Option<::common::archapi::VideoMode>,
	memmap: &'static [::memory::MemoryMapEnt],
}

enum BootInfo
{
	BootUninit,
	BootInvalid,
	BootMultiboot(MultibootParsed),
}


extern "C"
{
	static s_multiboot_signature : u32;
	static s_multiboot_pointer : &'static MultibootInfo;
}
static mut s_memmap_data: [::memory::MemoryMapEnt, ..16] = [::memory::MAP_PAD, ..16];
static mut s_bootinfo : BootInfo = BootUninit;

fn get_bootinfo() -> &'static BootInfo
{
	unsafe
	{
		match s_bootinfo
		{
		BootUninit => {
			s_bootinfo = match s_multiboot_signature
				{
				0x2BADB002 => BootMultiboot( MultibootParsed::new(s_multiboot_pointer) ),
				_ => BootInvalid,
				};
			},
		_ => {}
		}
		
		&s_bootinfo
	}
}

impl BootInfo
{
	pub fn cmdline(&self) -> &'static str
	{
		match *self
		{
		BootUninit => "",
		BootInvalid => "",
		BootMultiboot(ref mb) => mb.cmdline
		}
	}
	
	pub fn vidmode(&self) -> Option<::common::archapi::VideoMode>
	{
		match *self
		{
		BootUninit => None,
		BootInvalid => None,
		BootMultiboot(ref mb) => mb.vidmode
		}
	}
	pub fn memmap(&self) -> &'static[::memory::MemoryMapEnt]
	{
		match *self
		{
		BootUninit => [].as_slice(),
		BootInvalid => [].as_slice(),
		BootMultiboot(ref mb) => mb.memmap
		}
	}
}

impl MultibootParsed
{
	pub fn new(info: &MultibootInfo) -> MultibootParsed
	{
		let loader_ptr = (info.boot_loader_name as uint + IDENT_START) as *const i8;
		log_debug!("loader_ptr = {}", loader_ptr);
		let loader_name = if (info.flags & 1 << 9) != 0 && ::memory::c_string_valid(loader_ptr) {
				unsafe{ ::core::str::raw::c_str_to_static_slice( loader_ptr ) }
			}
			else {
				"-UNKNOWN-"
			};
		log_notice!("Loading multiboot from loader '{:s}'", loader_name);
		let mut ret = MultibootParsed {
				cmdline: MultibootParsed::_cmdline(info),
				vidmode: MultibootParsed::_vidmode(info),
				memmap: unsafe { &s_memmap_data },
			};
 		ret.memmap = unsafe { ret._memmap(info, &mut s_memmap_data) };
		ret
	}
	
	fn _cmdline(info: &MultibootInfo) -> &'static str
	{
		if (info.flags & 1 << 2) == 0 {
			return "";
		}
		
		let cmdline_paddr = info.cmdline as uint;
		if cmdline_paddr + IDENT_START >= IDENT_END {
			return "";
		}
		
		unsafe {
			let charptr = (cmdline_paddr + IDENT_START) as *const i8;
			::core::str::raw::c_str_to_static_slice( charptr )
		}
	}
	
	fn _vidmode(info: &MultibootInfo) -> Option<::common::archapi::VideoMode>
	{
		if (info.flags & 1 << 11) == 0 {
			log_notice!("get_video_mode - Video mode information not present");
			return None;
		}
		
		let vbeinfo_vaddr = info.vbe_mode_info as uint + IDENT_START;
		if vbeinfo_vaddr + ::core::mem::size_of::<VbeModeInfo>() > IDENT_END {
			return None;
		}
		
		let info: &VbeModeInfo = unsafe {
			::core::mem::transmute(vbeinfo_vaddr as *const VbeModeInfo)
			};
		
		Some( ::common::archapi::VideoMode {
			width: info.x_res,
			height: info.y_res,
			fmt: ::common::archapi::VideoX8R8G8B8,
			})
	}
	fn _memmap<'a>(&self, info: &MultibootInfo, buf: &'a mut[::memory::MemoryMapEnt]) -> &'a [::memory::MemoryMapEnt]
	{
		let size = {
			let mut mapbuilder = ::memory::MemoryMapBuilder::new(buf);
			// 1. Get raw map
			if false && (info.flags & 1 << 6) != 0 {
				// Full memory map
				fail!("TODO: Full memory map");
			}
			else if (info.flags & 1 << 0) != 0 {
				// Dumb memory map
				log_debug!("info = {{..., .lomem={}, .himem={} }}", info.lomem, info.himem);
				// - Low memory (before VGA BIOS)
				mapbuilder.append( 0, info.lomem as u64 * 1024, ::memory::StateFree, 0 );
				// - High memory (above 1MiB)
				mapbuilder.append( 0x100000, info.himem as u64 * 1024, ::memory::StateFree, 0 );
			}
			else {
				// No memory map
				fail!("TODO: Assumption memory map");
			}
			mapbuilder.sort();
			// TODO: Fix if not valid
			assert!( mapbuilder.validate() );
			
			// 2. Clobber out kernel, modules, and strings
			mapbuilder.set_range( 0x100000, &::arch::v_kernel_end as *const() as u64 - IDENT_START as u64 - 0x10000,
				::memory::StateUsed, 0 ).unwrap();
			mapbuilder.set_range( self.cmdline.as_ptr() as u64 - IDENT_START as u64, self.cmdline.len() as u64,
				::memory::StateUsed, 0 ).unwrap();
			
			mapbuilder.size()
			};
		
		// 3. Return final result
		buf.slice(0, size)
	}
}

// Retreive the multiboot "command line" string
pub fn get_boot_string() -> &'static str
{
	get_bootinfo().cmdline()
}

pub fn get_video_mode() -> Option<::common::archapi::VideoMode>
{
	get_bootinfo().vidmode()
}

pub fn get_memory_map() -> &'static[::memory::MemoryMapEnt]
{
	get_bootinfo().memmap()
}

// vim: ft=rust

