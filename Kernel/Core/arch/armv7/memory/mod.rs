

/*
pub type PAddrInner = u32;
*/

pub type PAddr = u32;
pub type VAddr = usize;

pub const PAGE_SIZE: usize = 0x2000;

pub mod virt;

pub mod addresses {
	pub fn is_global(addr: usize) -> bool {
		// Kernel area is global (i.e. present in all address spaces)
		addr >= KERNEL_BASE
	}

	pub const IDENT_SIZE: usize = 8*1024*1024;	
	
	pub const USER_END: usize = 0x8000_0000;
	pub const KERNEL_BASE: usize = 0x8000_0000;
	pub const HEAP_START: usize = 0x808_00000;	// 8MB because of the 8KB page size
	pub const HEAP_END  : usize = 0x8C0_00000;
	
	pub const BUMP_START: usize = 0x8C0_00000;
	pub const BUMP_END  : usize = 0xA00_00000;
	
	pub const HARDWARE_BASE: usize = 0xA00_00000;	
	pub const HARDWARE_END : usize = 0xB00_00000;	

	const MAX_RAM_BYTES: usize = 2*1024*1024*1024;
	pub const PMEMREF_BASE: usize = 0xB00_00000;
	pub const PMEMREF_END : usize = PMEMREF_BASE + MAX_RAM_BYTES / super::PAGE_SIZE * 4;	// 4 bytes / 8KB frame = 1MB?
	pub const PMEMBM_BASE: usize = PMEMREF_END;
	pub const PMEMBM_END : usize = PMEMBM_BASE + MAX_RAM_BYTES / super::PAGE_SIZE / 8;	// One bit per page
	

	pub const TEMP_BASE: usize = 0xEFF_00000;
	pub const TEMP_END : usize = 0xF00_00000;
	
	pub const STACKS_BASE: usize = 0xF00_00000;
	pub const STACKS_END: usize  = 0xF80_00000;
	pub const STACK_SIZE: usize = 4*super::PAGE_SIZE;
}

