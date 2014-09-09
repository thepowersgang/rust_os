// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/boot.rs
// - Boot information

use core::option::{Option,None,Some};
use super::memory::addresses::{ident_start, ident_end};
use super::puts;

#[repr(C)]
struct MultibootInfo
{
	flags: u32,
	// flags[0]
	himem: u32, lomem: u32,
	// flags[1]
	bootdev: u32,
	// flags[2]
	cmdline: u32,
	// flags[3]
	module_count: u32, module_first: u32,
	// flags[4] or flags[5]
	syminfo: [u32,..4],
	// flags[6]
	memmap_len: u32,
	memmap_ptr: u32,
	// flags[7]
	drives_length: u32,
	drives_addr: u32,
	// flags[8]
	configtable_ptr: u32,	// result of BIOS 'GET CONFIGURATION'
	// flags[9]
	boot_loader_name: u32,	// C string, booting loader
	// flags[10]
	apm_table_ptr: u32,
	// flags[11]
	vbe_control_info: u32,
	vbe_mode_info: u32,
	vbe_mode: u32,
	vbe_interface_seg: u32,
	vbe_interface_off: u32,
	vbe_interface_len: u32,
}

#[repr(C)]
struct VbeModeInfo
{
	attributes: u16,
	window_attrs: [u8,..2],
	granuality: u16,
	window_size: u16,
	window_segments: [u16, ..2],
	win_pos_fcn_fptr: [u16, ..2],	// Pointer to INT 10h, AX=4F05h
	
	pitch: u16,
	x_res: u16, y_res: u16,
	char_w: u8, char_h: u8,
	n_planes: u8,
	bpp: u8,
	n_banks: u8,
	memory_model: u8,
	bank_size: u8,
	n_pages: u8,
	_resvd: u8,	// reserved
	
	// VBE 1.2+
	
}

extern "C"
{
	static s_multiboot_signature : u32;
	static s_multiboot_pointer : &'static MultibootInfo;
}

// Retreive the multiboot "command line" string
pub fn get_boot_string() -> &'static str
{
	//::puts("Multiboot signature: "); ::puth(s_multiboot_signature as uint); ::puts("\n");
	//::puts("Multiboot pointer: "); ::puth(s_multiboot_pointer as uint); ::puts("\n");
	if s_multiboot_signature != 0x2BADB002 {
		return "";
	}
	//::puts("> Flags = "); ::puth(s_multiboot_pointer.flags as uint); ::puts("\n");
	if (s_multiboot_pointer.flags & 1 << 2) == 0 {
		return "";
	}
	//::puts("> cmdline = "); ::puth(s_multiboot_pointer.cmdline as uint); ::puts("\n");
	
	let cmdline_paddr = s_multiboot_pointer.cmdline as uint;
	if cmdline_paddr + ident_start >= ident_end {
		return "";
	}
	
	unsafe {
		let charptr : *const i8 = ::core::mem::transmute( cmdline_paddr + ident_start );
		::core::str::raw::c_str_to_static_slice( charptr )
	}
}

pub fn get_video_mode() -> Option<::common::archapi::VideoMode>
{
	if s_multiboot_signature != 0x2BADB002 {
		puts("arch::boot::get_video_mode - Multiboot signature not valid\n");
		return None;
	}
	if (s_multiboot_pointer.flags & 1 << 11) == 0 {
		puts("arch::boot::get_video_mode - Video mode information not present\n");
		return None;
	}
	
	let vbeinfo_vaddr = s_multiboot_pointer.vbe_mode_info as uint + ident_start;
	if vbeinfo_vaddr + ::core::mem::size_of::<VbeModeInfo>() > ident_end {
		return None;
	}
	
	let info: &VbeModeInfo = unsafe {
		::core::mem::transmute(vbeinfo_vaddr as *const VbeModeInfo)
		};
	
	Some( ::common::archapi::VideoMode {
		width: info.x_res,
		height: info.y_res,
		fmt: ::common::archapi::VideoX8R8G8B8,
		})
}

// vim: ft=rust

