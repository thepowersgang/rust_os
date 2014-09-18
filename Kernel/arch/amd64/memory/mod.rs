//
//
//
pub static PAGE_SIZE: uint = 0x1000;

pub type PAddr = u64;
pub type VAddr = uint;

pub mod addresses
{
	pub static heap_start:     uint = 0xFFFF8000_00000000;
	pub static heap_end:       uint = 0xFFFF9000_00000000;
	pub static modules_start:  uint = heap_end;
	pub static modules_end:    uint = 0xFFFFA000_00000000;
	pub static hardware_base:  uint = modules_end;
	pub static hardware_end:   uint = 0xFFFFB000_00000000;
	//pub static physinfo_start: uint = 0xFFFFA000_00000000;
	//pub static physinfo_end:   uint = 0xFFFFB000_00000000;	// TODO: Needed?
	pub static fractal_base:   uint = 0xFFFFFE00_00000000;	// PML4[508]
	pub static ident_start:    uint = 0xFFFFFFFF_80000000;
	pub static ident_end:      uint = ident_start + 0x200000;	// 2MiB
	
	pub fn is_global(addr: uint) -> bool
	{
		if addr < heap_start {
			return false;
		}
		// TODO: Kernel-side per-process data
		return true;
	}
}

pub mod virt;

// vim: ft=rust
