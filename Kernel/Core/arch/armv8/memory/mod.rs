//
//
//
//
pub const PAGE_SIZE: usize = 0x4000;	// Architecture supports 4K/16K/64K, pick something different to catch bugs
pub type VAddr = usize;
pub type PAddr = u64;

pub mod addresses {
	pub const USER_END : usize = 0x0000_7FF0_0000_0000;	// Leaves some space for user fractal
	pub(super) const USER_FRACTAL_BASE: usize = 0x0000_7FF0_0000_0000;

	pub const IDENT_START: usize = 0xFFFF_8000_0000_0000;
	pub const IDENT_SIZE : usize = 0x0000_0000_0200_0000;

	pub const HEAP_START: usize = 0xFFFF_8000_0200_0000;
	pub const HEAP_END  : usize = 0xFFFF_C000_0000_0000;

	// Physical memory reference counting base:
	//  - D-C = 1<<(32+14) = (1 << 46)
	//  - / 4 = (1 << 44) frames, = 16 trillion = 16PB RAM
	pub const PMEMREF_BASE:   usize = HEAP_END;
	pub const PMEMREF_END:    usize = 0xFFFF_D000_00000000;
	const MAX_FRAME_IDX: usize = (PMEMREF_END - PMEMREF_BASE) / 4;	// 32-bit integer each
	pub const PMEMBM_BASE:	  usize = PMEMREF_END;
	pub const PMEMBM_END:     usize = PMEMBM_BASE + MAX_FRAME_IDX / 8;	// 8 bits per byte in bitmap

	pub const BUMP_START: usize = 0xFFFF_FFA0_0000_0000;
	pub const BUMP_END  : usize = 0xFFFF_FFB0_0000_0000;
	pub const STACKS_BASE: usize = 0xFFFF_FFB0_0000_0000;
	pub const STACKS_END : usize = 0xFFFF_FFC0_0000_0000;
	pub const STACK_SIZE: usize = 0x8000;	// one page data, one page guard
	pub const HARDWARE_BASE: usize = 0xFFFF_FFC0_0000_0000;
	pub const HARDWARE_END : usize = 0xFFFF_FFD0_0000_0000;

	pub const KERNEL_FRACTAL_BASE: usize = 0xFFFF_FFE0_0000_0000;

	pub const TEMP_BASE: usize = 0xFFFF_FFF0_0000_0000;
	pub const TEMP_END: usize = 0xFFFF_FFF0_0200_0000;	// 2048 (0x800) slots = 0x4*0x8*0x1000*0x100 = 0x20_000_00

	pub fn is_global(addr: usize) -> bool {
		addr >= 0xFFFF_0000_0000_0000
	}
}

pub mod virt;

