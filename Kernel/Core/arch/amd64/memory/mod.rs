// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/memory/mod.rs
//! Architecture-level memory management and definitions.

/// Size of a page.
pub const PAGE_SIZE: usize = 0x1000;

/// Physical address type
pub type PAddr = u64;
/// Virtual address type (TODO: Remove this, usize is defined to be this)
pub type VAddr = usize;

pub mod addresses
{
	//! Fixed addresses
	
	pub const USER_END:       usize = 0x00008000_00000000;
	
	/// Start of the kernel heap
	pub const HEAP_START:     usize = 0xFFFF8000_00000000;
	/// End of the kernel heap
	pub const HEAP_END:       usize = 0xFFFF9000_00000000;
	
	/// Start of the kernel module load area
	pub const MODULES_BASE:   usize = HEAP_END;
	/// End of the kernel module load area
	pub const MODULES_END:    usize = 0xFFFFA000_00000000;
	
	/// Start of the stacks region
	pub const STACKS_BASE:    usize = MODULES_END;
	/// End of the stacks region
	pub const STACKS_END:     usize = 0xFFFFB000_00000000;
	
	/// Start of the hardware mapping region
	pub const HARDWARE_BASE:  usize = STACKS_END;
	/// End of the hardware mapping region
	pub const HARDWARE_END:   usize = 0xFFFF_C000_00000000;

	// Physical memory reference counting base:
	//  - D-C = 1<<(32+12) = (1 << 44)
	//  - / 4 = (1 << 42) frames, = 4 trillion = 16PB RAM
	pub const PMEMREF_BASE:   usize = HARDWARE_END;
	pub const PMEMREF_END:    usize = 0xFFFF_D000_00000000;
	const MAX_FRAME_IDX: usize = (PMEMREF_END - PMEMREF_BASE) / 4;	// 32-bit integer each
	pub const PMEMBM_BASE:	  usize = PMEMREF_END;
	pub const PMEMBM_END:     usize = PMEMBM_BASE + MAX_FRAME_IDX / 8;	// 8 bits per byte in bitmap
	
	pub const BUMP_START:	usize = 0xFFFF_E000_00000000;
	pub const BUMP_END:	usize = 0xFFFF_F000_00000000;
	// Most of F is free
	
	pub const STACK_SIZE: usize = 0x8000;   // 4pg allocation was overflowed, 8 works
	
	#[doc(hidden)]
	/// Start of the fractal mapping
	pub const FRACTAL_BASE:    usize = 0xFFFFFE00_00000000;	// PML4[508]
	#[doc(hidden)]
	pub const IDENT_START:    usize = 0xFFFFFFFF_80000000;	// PML4[511] (plus some)
	#[doc(hidden)]
	pub const IDENT_END:      usize = IDENT_START + 0x400000;	// 4MiB
	
	/// 
	pub const TEMP_BASE: usize = IDENT_END;
	pub const TEMP_END:  usize = 0xFFFFFFFF_FFFF0000;	// Leave the last 16 pages free
	
	/// returns true if the provided address is valid within all address spaces
	pub fn is_global(addr: usize) -> bool
	{
		if addr < USER_END {
			false
		}
		else if addr < HEAP_START {
			panic!("Calling is_global on non-canonical address {:#x}", addr)
		}
		// TODO: Kernel-side per-process data
		else {
			true
		}
	}
}

pub mod virt;

// vim: ft=rust
