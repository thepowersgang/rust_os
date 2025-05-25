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
		multiboot::MAGIC =>
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

pub fn release_preboot_video() {
}
const PREBOOT_VIDEO_STATUS_UNINIT: u8 = 0;
const PREBOOT_VIDEO_STATUS_UNDETERMINED: u8 = 1;
const PREBOOT_VIDEO_STATUS_ENABLED: u8 = 2;
const PREBOOT_VIDEO_STATUS_RELEASED: u8 = 3;
static PREBOOT_VIDEO_STATUS: ::core::sync::atomic::AtomicU8 = ::core::sync::atomic::AtomicU8::new(0);
static PREBOOT_VIDEO_FMT: ::core::sync::atomic::AtomicU32 = ::core::sync::atomic::AtomicU32::new(0);
static PREBOOT_VIDEO_PTR: ::core::sync::atomic::AtomicPtr<u32> = ::core::sync::atomic::AtomicPtr::new(0 as *mut _);
pub(super) fn with_preboot_video(f: impl FnOnce(&mut [u32], usize)) {
	extern "C" {
		static BootHwPD0: [::core::sync::atomic::AtomicU64; 512];
	}
	use ::core::sync::atomic::Ordering;
	let sts = match PREBOOT_VIDEO_STATUS.compare_exchange(PREBOOT_VIDEO_STATUS_UNINIT, PREBOOT_VIDEO_STATUS_UNDETERMINED, Ordering::Relaxed, Ordering::Relaxed)
		{
		Ok(v) => v,
		Err(v) => v,
		};
	match sts {
	// SAFE: Checked - passing a valid boot status pointer, and the generated slice is good.. enough
	PREBOOT_VIDEO_STATUS_UNINIT|PREBOOT_VIDEO_STATUS_ENABLED => unsafe {
		if sts == PREBOOT_VIDEO_STATUS_UNINIT {
			let vi = match s_multiboot_signature {
				multiboot::MAGIC => multiboot::get_video(s_multiboot_pointer),
				_ => None,
			};
			match vi {
			Some(mode) if 
				true
				&& mode.pitch == 4 * mode.width as usize
				//&& mode.height < 0x1_0000
				//&& mode.width < 0x1_0000
				&& matches!(mode.fmt, crate::metadevs::video::bootvideo::VideoFormat::X8R8G8B8)
			=> {
				//const MAP_SIZE: u64 = 0x4000_0000;
				const MAP_SIZE: u64 = 0x20_0000;
				let phys_ofs = mode.base & (MAP_SIZE-1);
				let phys_aligned = mode.base & !(MAP_SIZE-1);
				let size = mode.width as usize * mode.height as usize * 4;
				let n_maps = (size as u64 + MAP_SIZE-1) / MAP_SIZE;
				for i in 0 .. n_maps {
					BootHwPD0[i as usize].store((phys_aligned + i * MAP_SIZE) | 0x83, Ordering::Relaxed);
				}
				PREBOOT_VIDEO_PTR.store( (0xFFFF_F000_0000_0000 + phys_ofs) as *mut u32, Ordering::Relaxed );
				PREBOOT_VIDEO_FMT.store( mode.width as u32 | (mode.height as u32) << 16, Ordering::Relaxed );
				PREBOOT_VIDEO_STATUS.store(PREBOOT_VIDEO_STATUS_ENABLED, Ordering::Release);
			}
			_ => {
				PREBOOT_VIDEO_STATUS.store(PREBOOT_VIDEO_STATUS_RELEASED, Ordering::Relaxed);
				return ;
			}
			}
		}
		let size_enc = PREBOOT_VIDEO_FMT.load(Ordering::Relaxed);
		let w = size_enc & 0xFFFF;
		let h = size_enc >> 16;
		let n_px = w as usize * h as usize;
		let ptr = PREBOOT_VIDEO_PTR.load(Ordering::Relaxed);
		// TODO: This isn't strictly speaking unique, but let's assume that it's valid enough
		// - The caller of this is wrapped in a lock anyway
		f( ::core::slice::from_raw_parts_mut(ptr, n_px), w as usize );
		},
	PREBOOT_VIDEO_STATUS_RELEASED => {},
	PREBOOT_VIDEO_STATUS_UNDETERMINED => {},
	_ => {},
	}
}

// vim: ft=rust