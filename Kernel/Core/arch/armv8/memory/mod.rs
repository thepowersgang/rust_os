//
//
//
//
pub const PAGE_SIZE: usize = 0x4000;
pub type VAddr = usize;
pub type PAddr = u64;

pub mod addresses {
	pub const USER_END: usize = 0x0000_FFF0_0000_0000;

	pub const IDENT_START: usize = 0xFFFF_8000_0000_0000;
	pub const IDENT_SIZE : usize = 0x0000_0000_0200_0000;

	pub const HEAP_START: usize = 0xFFFF_8000_0200_0000;
	pub const HEAP_END  : usize = 0xFFFF_C000_0000_0000;

	pub const BUMP_START: usize = 0xFFFF_FFA0_0000_0000;
	pub const BUMP_END  : usize = 0xFFFF_FFB0_0000_0000;
	pub const STACKS_BASE: usize = 0xFFFF_FFB0_0000_0000;
	pub const STACKS_END : usize = 0xFFFF_FFC0_0000_0000;
	pub const HARDWARE_BASE: usize = 0xFFFF_FFC0_0000_0000;
	pub const HARDWARE_END : usize = 0xFFFF_FFD0_0000_0000;

	pub const STACK_SIZE: usize = 0x8000;

	pub fn is_global(addr: usize) -> bool {
		addr >= 0xFFFF_0000_0000_0000
	}
}

pub mod virt;
pub mod phys;

