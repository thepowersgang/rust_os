
use metadevs::video::bootvideo::{VideoMode,VideoFormat};

pub fn get_video_mode() -> Option<VideoMode> {
	None
}

pub fn get_boot_string() -> &'static str {
	""
}

pub fn get_memory_map() -> &'static[::memory::MemoryMapEnt] {
	&[]
}

