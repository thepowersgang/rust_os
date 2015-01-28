//
//
//
pub const PAGE_SIZE: usize = 0x1000;

pub type PAddr = u64;
pub type VAddr = usize;

pub mod addresses
{
	pub const HEAP_START:     usize = 0xFFFF8000_00000000;
	pub const HEAP_END:       usize = 0xFFFF9000_00000000;
	pub const MODULES_START:  usize = HEAP_END;
	pub const MODULES_END:    usize = 0xFFFFA000_00000000;
	pub const HARDWARE_BASE:  usize = MODULES_END;
	pub const HARDWARE_END:   usize = 0xFFFFB000_00000000;
	//pub const physinfo_start: usize = 0xFFFFA000_00000000;
	//pub const physinfo_end:   usize = 0xFFFFB000_00000000;	// TODO: Needed?
	pub const FRACTAL_BASE:   usize = 0xFFFFFE00_00000000;	// PML4[508]
	pub const IDENT_START:    usize = 0xFFFFFFFF_80000000;
	pub const IDENT_END:      usize = IDENT_START + 0x200000;	// 2MiB
	
	pub fn is_global(addr: usize) -> bool
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
