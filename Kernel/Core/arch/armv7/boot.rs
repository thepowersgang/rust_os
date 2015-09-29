
use metadevs::video::bootvideo::{VideoMode,VideoFormat};
use lib::lazy_static::LazyStatic;
use super::fdt::FDTRoot;

extern "C" {
	static dt_base: [u32; 2];
	static mut kernel_exception_map: [u32; 1024];
}

static S_FDT: LazyStatic< FDTRoot<'static> > = LazyStatic::new();

pub fn get_video_mode() -> Option<VideoMode> {
	None
}

pub fn get_boot_string() -> &'static str {
	""
}

pub fn get_memory_map() -> &'static[::memory::MemoryMapEnt] {
	if dt_base[0] == 0 {
		// Ok... we know very little about the memory layout
	}
	else {
		// Firmware device tree present!
		// TODO: A more permanent solution to this
		// SAFE: (Well, as can be) No aliasing, and should stay valid
		let fdt = unsafe {
			kernel_exception_map[1024-3] = dt_base[0] + 0x13;
			kernel_exception_map[1024-2] = dt_base[0] + 0x1000 + 0x13;
			super::fdt::FDTRoot::new_raw(0xFFFFD000 as *const u8)
			};
		fdt.dump_nodes();
		//log_debug!("ftd = {:?}", fdt);
	}
	&[]
}

