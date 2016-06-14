

/*
pub type PAddrInner = u32;
*/

pub type PAddr = u32;
pub type VAddr = usize;

pub const PAGE_SIZE: usize = 0x2000;

pub mod virt;
pub mod phys;

pub mod addresses {
	pub fn is_global(addr: usize) -> bool {
		// Kernel area is global (i.e. present in all address spaces)
		addr >= KERNEL_BASE
	}
	
	
	pub const USER_END: usize = 0x8000_0000;
	pub const KERNEL_BASE: usize = 0x8000_0000;
	pub const HEAP_START: usize = 0x808_00000;	// 8MB because of the 8KB page size
	pub const HEAP_END  : usize = 0x8C0_00000;
	
	pub const BUMP_START: usize = 0x8C0_00000;
	pub const BUMP_END  : usize = 0xA00_00000;
	
	pub const HARDWARE_BASE: usize = 0xA00_00000;	
	pub const HARDWARE_END : usize = 0xB00_00000;	
	pub const TEMP_BASE: usize = 0xEFF_00000;
	pub const TEMP_END : usize = 0xF00_00000;
	
	pub const STACKS_BASE: usize = 0xF00_00000;
	pub const STACKS_END: usize  = 0xF80_00000;
	pub const STACK_SIZE: usize = 4*::PAGE_SIZE;
}

