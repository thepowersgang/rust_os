// "Tifflin" Kernel
// - By John Hodge (Mutabah/thePowersGang)
//
// Core/arch/riscv64/memory.rs
//! RISC-V boot handling
// TODO: This is almost idential to ARMv8

use crate::lib::lazy_static::LazyStatic;
use crate::lib::fdt::FDTRoot;

pub fn get_fdt() -> Option<&'static FDTRoot<'static>> {
	Some(get_fdt_real())
}
fn get_fdt_real() -> &'static FDTRoot<'static>
{
	static S_FDT: LazyStatic<FDTRoot<'static>> = LazyStatic::new();
	// SAFE: Correct accesses to extern data, correct ue of FDTRoot::new_raw
	S_FDT.prep(|| unsafe {
		extern "C" {
			static fdt_phys_addr: u64;
			static mut boot_pt_lvl2_hwmaps: [u64; 512];
		}
		log_debug!("fdt_phys_addr = {:#x}", fdt_phys_addr);
		boot_pt_lvl2_hwmaps[1] = ((fdt_phys_addr & !(0x20_0000-1)) >> 2) | (1 << 1) | 1;
		let fdt_addr: usize = 0xFFFFFFFF_40000000 + 0x20_0000 + (fdt_phys_addr as usize & (0x20_0000-1));
		let fdt = FDTRoot::new_raw(fdt_addr as *const u8);
		fdt.dump_nodes();
		fdt
		})
}

/// Obtain kernel command line
pub fn get_boot_string() -> &'static str {
	static S_BOOT_STRING: LazyStatic<&'static str> = LazyStatic::new();
	*S_BOOT_STRING.prep(|| {
		let fdt = get_fdt_real();
		fdt.get_props(&["", "chosen", "bootargs"]).next()
			.map(|v| if v.last() == Some(&0) { &v[..v.len()-1] } else { v })	// Should be NUL terminated
			.map(|v| ::core::str::from_utf8(v).expect("Boot arguments not valid UTF-8"))
			.unwrap_or("")
		})
}
/// qemu virt platform doesn't have a boot video mode
pub fn get_video_mode() -> Option<crate::metadevs::video::bootvideo::VideoMode> {
	None
}
/// Obtain/generate memory map
pub fn get_memory_map() -> &'static [crate::memory::MemoryMapEnt]
{
	// Statically allocated memory for the memory map
	static S_MEMORY_MAP: LazyStatic<(usize, [crate::memory::MemoryMapEnt; 10])> = LazyStatic::new();

	let (len, ref mm) = *S_MEMORY_MAP.prep(|| {
		let mut buf = [crate::memory::MAP_PAD; 10];

		let mut mapbuilder = crate::memory::MemoryMapBuilder::new(&mut buf);
		let fdt = get_fdt_real();

		// Build a map using RAM entries from the FDT
		for prop in fdt.get_props_cb(|idx,leaf,name| match (idx,leaf)
			{
			(0,false) => name == "",
			(1,false) => name == "memory" || name.starts_with("memory@"),
			(2,true) => name == "reg",
			_ => false,
			})
		{
			use crate::lib::byteorder::{ReadBytesExt,BigEndian};
			let mut p = prop;
			let base = p.read_u64::<BigEndian>().unwrap();
			let size = p.read_u64::<BigEndian>().unwrap();
			log_debug!("base = {:#x}, size = {:#x}", base, size);
			mapbuilder.append( base, size, crate::memory::MemoryState::Free, 0 );
		}
		// Mask out known-used areas
		// -- FDT
		extern "C" {
			static fdt_phys_addr: u64;
		}
		// SAFE: Immutable static
		mapbuilder.set_range( unsafe { fdt_phys_addr }, fdt.size() as u64, crate::memory::MemoryState::Used, 0 ).unwrap();

		// -- SBI image (TODO: Don't hard-code this)
		mapbuilder.set_range(0x8000_0000, 0x0020_0000, crate::memory::MemoryState::Used, 0).unwrap();

		// -- Kernel image (TODO: Can this be non-hard-coded too?)
		extern "C" {
			static _phys_base: crate::Extern;
			static _kernel_base: crate::Extern;
			static v_kernel_end: crate::Extern;
		}
		// SAFE: Taking the address only
		let (kernel_phys, kernel_len) = unsafe {
			//(&_phys_base as *const _ as usize as u64, &v_kernel_end as *const _ as usize - &_kernel_base as *const _ as usize)
			(0x8000_0000 + 0x0020_0000, &v_kernel_end as *const _ as usize - &_kernel_base as *const _ as usize)
			};
		mapbuilder.set_range(kernel_phys, kernel_len as u64, crate::memory::MemoryState::Used, 0).unwrap();

		let len = mapbuilder.size();
		(len, buf)
		});
	&mm[..len]
}
