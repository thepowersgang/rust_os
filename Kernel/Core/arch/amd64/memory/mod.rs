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
	pub const HARDWARE_END:   usize = 0xFFFFC000_00000000;
	
	/// 
	pub const TEMP_BASE: usize = HARDWARE_END;
	pub const TEMP_END:  usize = 0xFFFFD000_00000000;
	
	pub const STACK_SIZE: usize = 0x8000;   // 4pg allocation was overflowed, 8 works
	
	#[doc(hiddden)]
	/// Start of the fractal mapping
	pub const FRACTAL_BASE:    usize = 0xFFFFFE00_00000000;	// PML4[508]
	#[doc(hiddden)]
	pub const IDENT_START:    usize = 0xFFFFFFFF_80000000;	// PML4[511] (plus some)
	#[doc(hiddden)]
	pub const IDENT_END:      usize = IDENT_START + 0x200000;	// 2MiB
	
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
pub mod phys;

// vim: ft=rust
