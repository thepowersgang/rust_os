
use metadevs::video::bootvideo::{VideoMode,VideoFormat};
use lib::lazy_static::LazyStatic;
use super::fdt::FDTRoot;

extern "C" {
	static dt_base: u32;
	static kernel_data_start: u32;
	static mut kernel_exception_map: [u32; 1024];
	static data_len: ();
	static __bss_len: ();
}

enum BootInfo
{
	None,
	Basic(u32,u32),
	FDT(FDTRoot<'static>),
}

static S_FDT: LazyStatic<BootInfo> = LazyStatic::new();
static mut S_MEMMAP_DATA: [::memory::MemoryMapEnt; 16] = [::memory::MAP_PAD; 16];

fn get_boot_info() -> &'static BootInfo {
	if ! S_FDT.ls_is_valid() {
		// SAFE: Shouldn't be called in a racy manner
		unsafe { S_FDT.prep(|| BootInfo::new()) }
	}
	&S_FDT
}

impl BootInfo
{
	fn new() -> BootInfo {
		if dt_base == 0 {
			BootInfo::None
		}
		else {
			// SAFE: In practice, this is run in a single-thread. Any possible race would be benign
			unsafe {
				const FLAGS: u32 = 0x13;
				kernel_exception_map[1024-3] = dt_base + FLAGS;
				kernel_exception_map[1024-2] = dt_base + 0x1000 + FLAGS;
			}
			// SAFE: Memory is valid, and is immutable
			unsafe {
				BootInfo::FDT( super::fdt::FDTRoot::new_raw(0xFFFFD000 as *const u8) )
			}
		}
	}
}

pub fn get_video_mode() -> Option<VideoMode> {
	None
}

pub fn get_boot_string() -> &'static str {
	match get_boot_info()
	{
	&BootInfo::FDT(ref fdt) => fdt.get_props(&["","chosen","bootargs"]).next().map(|x| ::core::str::from_utf8(&x[..x.len()-1]).unwrap_or("") ).unwrap_or(""),
	_ => "",
	}
}

pub fn get_memory_map() -> &'static [::memory::MemoryMapEnt] {
	// TODO: Assert that this is only ever called once
	// SAFE: Assuming this function is called only once (which it is)
	let buf: &mut [_] = unsafe { &mut S_MEMMAP_DATA };
	let len = {
		let mut mapbuilder = ::memory::MemoryMapBuilder::new(buf);
		match get_boot_info()
		{
		&BootInfo::None => {},
		&BootInfo::Basic(ram_base, ram_len) => {
			mapbuilder.append( ram_base as u64, ram_len as u64, ::memory::MemoryState::Free, 0 );
			},
		&BootInfo::FDT(ref fdt) => {
			// FDT Present, need to locate all memory nodes
			fdt.dump_nodes();
			for prop in fdt.get_props(&["","memory","reg"])
			{
				use lib::byteorder::{ReadBytesExt,BigEndian};
				let mut p = prop;
				let base = p.read_u64::<BigEndian>().unwrap();
				let size = p.read_u64::<BigEndian>().unwrap();
				log_debug!("base = {:#x}, size = {:#x}", base, size);
				mapbuilder.append( base, size, ::memory::MemoryState::Free, 0 );
			}
			mapbuilder.set_range( dt_base as u64, fdt.size() as u64, ::memory::MemoryState::Used, 0 ).unwrap();
			},
		}

		if kernel_data_start != 0 {
			// 2. Clobber out kernel, modules, and strings
			mapbuilder.set_range( kernel_data_start as u64, (&data_len as *const _ as u64 + &__bss_len as *const _ as u64), ::memory::MemoryState::Used, 0 ).unwrap();
		}
		
		mapbuilder.size()
		};
	&buf[..len]
}

