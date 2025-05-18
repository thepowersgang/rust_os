// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/boot.rs
//! Boot information.
//!
//! Parsing and exposure of the bootloader-provided data
#[allow(unused_imports)]
use crate::prelude::*;
use crate::metadevs::video::bootvideo::VideoMode;
use crate::symbols::Elf32_Sym;
use crate::memory::MemoryMapEnt;
use crate::arch::boot::ModuleInfo;

mod multiboot;
use self::multiboot::MultibootParsed;
mod uefi;
use self::uefi::UefiParsed;

enum BootInfo
{
	Invalid,
	Multiboot(MultibootParsed),
	Uefi(UefiParsed),
}

enum SymbolInfo
{
	None,
	Elf32( &'static [Elf32_Sym], &'static [u8] ),
}

extern "C"
{
	static s_multiboot_signature : u32;
	static s_multiboot_pointer : *const crate::Void;
}

#[allow(static_mut_refs)]	// Used in a safe manner, and I CBF wrapping it up
fn get_bootinfo() -> &'static BootInfo
{
	static S_BOOTINFO: crate::lib::LazyStatic<BootInfo> = crate::lib::LazyStatic::new();
	static mut S_MEMMAP_DATA: [MemoryMapEnt; 16] = [crate::memory::MAP_PAD; 16];
	static mut S_MODULES_DATA: [ModuleInfo; 16] = [ModuleInfo::EMPTY; 16];
	// SAFE: Correct use of `extern static` (data is read-only once out of assembly stub)
	// SAFE: `static mut` is only referenced here, inside a concurrency-protected function
	S_BOOTINFO.prep(|| unsafe {
		let info_ptr = s_multiboot_pointer;
		match s_multiboot_signature
		{
		0x2BADB002 =>
			if let Some(mbi) = MultibootParsed::from_ptr(info_ptr, &mut S_MEMMAP_DATA, &mut S_MODULES_DATA ) {
				BootInfo::Multiboot(mbi)
			}
			else {
				BootInfo::Invalid
			},
		uefi::MAGIC =>
			if let Some(i) = UefiParsed::from_ptr(info_ptr, &mut S_MEMMAP_DATA) {
				BootInfo::Uefi(i)
			}
			else {
				BootInfo::Invalid
			},
		_ => BootInfo::Invalid,
		}
		})
}

impl BootInfo
{
	pub fn cmdline(&self) -> &'static str
	{
		match *self
		{
		BootInfo::Invalid => "",
		BootInfo::Multiboot(ref i) => i.cmdline,
		BootInfo::Uefi(ref i) => i.cmdline,
		}
	}
	
	pub fn vidmode(&self) -> Option<VideoMode>
	{
		match *self
		{
		BootInfo::Invalid => None,
		BootInfo::Multiboot(ref i) => i.vidmode,
		BootInfo::Uefi(ref i) => i.vidmode,
		}
	}
	pub fn memmap(&self) -> &'static [MemoryMapEnt]
	{
		match *self
		{
		BootInfo::Invalid => &[],
		BootInfo::Multiboot(ref i) => i.memmap,
		BootInfo::Uefi(ref i) => i.memmap,
		}
	}
	pub fn modules(&self) -> &'static [ModuleInfo]
	{
		match *self
		{
		BootInfo::Invalid => &[],
		BootInfo::Multiboot(ref i) => i.modules,
		BootInfo::Uefi(_) => &[],
		}
	}
}

unsafe fn valid_c_str_to_slice(ptr: *const i8) -> Option<&'static str> {
	if let Some(s) = crate::memory::c_string_as_byte_slice(ptr) {
		::core::str::from_utf8(s).ok()
	}
	else {
		None
	}
}


/// Retrieve the multiboot "command line" string
pub fn get_boot_string() -> &'static str {
	get_bootinfo().cmdline()
}

/// Obtain the boot video mode
pub fn get_video_mode() -> Option<VideoMode> {
	get_bootinfo().vidmode()
}

/// Obtain the memory map
pub fn get_memory_map() -> &'static [MemoryMapEnt] {
	get_bootinfo().memmap()
}

/// Obtain the bootloader-provided modules
pub fn get_modules() -> &'static [ModuleInfo] {
	get_bootinfo().modules()
}

// vim: ft=rust