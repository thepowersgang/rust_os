//
//
//
pub const PAGE_SIZE: uint = 0x1000;

pub type PAddr = u64;
pub type VAddr = uint;

pub mod addresses
{
	pub const HEAP_START:     uint = 0xFFFF8000_00000000;
	pub const HEAP_END:       uint = 0xFFFF9000_00000000;
	pub const MODULES_START:  uint = HEAP_END;
	pub const MODULES_END:    uint = 0xFFFFA000_00000000;
	pub const HARDWARE_BASE:  uint = MODULES_END;
	pub const HARDWARE_END:   uint = 0xFFFFB000_00000000;
	//pub const physinfo_start: uint = 0xFFFFA000_00000000;
	//pub const physinfo_end:   uint = 0xFFFFB000_00000000;	// TODO: Needed?
	pub const FRACTAL_BASE:   uint = 0xFFFFFE00_00000000;	// PML4[508]
	pub const IDENT_START:    uint = 0xFFFFFFFF_80000000;
	pub const IDENT_END:      uint = IDENT_START + 0x200000;	// 2MiB
	
	pub fn is_global(addr: uint) -> bool
	{
		if addr < HEAP_START {
			return false;
		}
		// TODO: Kernel-side per-process data
		return true;
	}
}

pub mod virt;

// vim: ft=rust
