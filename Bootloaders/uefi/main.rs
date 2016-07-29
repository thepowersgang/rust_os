#![feature(lang_items)]
#![feature(asm)]
#![no_std] 
//#![crate_type="lib"]

use uefi::boot_services::protocols;

#[macro_use]
extern crate uefi;

macro_rules! log {
	($($v:tt)*) => { loge!( ::get_conout(), $($v)*) };
}
#[path="../_common/elf.rs"]
mod elf;

//static PATH_CONFIG: &'static [u16] = ucs2_c!("Tifflin\\boot.cfg");
//static PATH_FALLBACK_KERNEL: &'static [u16] = ucs2_c!("Tifflin\\kernel-amd4.bin");
macro_rules! u16_cs {
	($($v:expr),+) => ( [$($v as u16),*] );
}
static PATH_CONFIG: &'static [u16] = &u16_cs!('T','I','F','F','L','I','N','\\','B','O','O','T','.','C','F','G',0);
static PATH_FALLBACK_KERNEL: &'static [u16] = &u16_cs!('T','I','F','F','L','I','N','\\','K','E','R','N','E','L','.','E','L','F',0);

// Marker to tell where the executable was loaded
#[link_section=".text"]
static S_MARKER: () = ();

static mut S_CONOUT: *const ::uefi::SimpleTextOutputInterface = 1 as *const _;

pub fn get_conout() -> &'static ::uefi::SimpleTextOutputInterface {
	unsafe { &*S_CONOUT }
}

#[no_mangle]
pub extern "win64" fn efi_main(image_handle: ::uefi::Handle, system_table: &::uefi::SystemTable) -> ::uefi::Status
{
	// SAFE: Assuming that the system table data is valid
	let conout = system_table.con_out();
	unsafe {
		S_CONOUT = conout;
	}
	loge!(conout, "efi_main(image_handle={:?}, system_table={:p}) - {:p}", image_handle, system_table, &S_MARKER);
	//let sp = unsafe { let v: u64; asm!("mov %rsp, $0" : "=r" (v)); v };
	//loge!(conout, "- RSP: {:p}", sp as usize as *const ());
	loge!(conout, "- Firmware Version {:#x} by '{}'", system_table.firmware_revision, system_table.firmware_vendor());
	loge!(conout, "- Boot Services @ {:p}, Runtime Services @ {:p}",
		system_table.boot_services, system_table.runtime_services);
	
	let image_dev: &protocols::LoadedImageDevicePath = system_table.boot_services.handle_protocol(&image_handle).expect("image_handle - LoadedImageDevicePath");
	//loge!(conout, "- image_dev = {:?}", image_dev);
	let image_proto: &protocols::LoadedImage = system_table.boot_services.handle_protocol(&image_handle).expect("image_handle - LoadedImage");
	//loge!(conout, "- image_proto.file_path={:?}", image_proto.file_path);
	
	if image_proto.file_path.type_code() != (4,4) {
	}

	let system_volume_fs: &protocols::SimpleFileSystem = system_table.boot_services.handle_protocol(&image_proto.device_handle).expect("image_proto - FileProtocol");
	let system_volume_root = system_volume_fs.open_volume().expect("system_volume_fs - File");
	
	let kernel_file = match system_volume_root.open_read(PATH_CONFIG)
		{
		Ok(cfg) => panic!("TODO: Read config file"),
		Err(::uefi::status::NOT_FOUND) => {
			system_volume_root.open_read(PATH_FALLBACK_KERNEL).expect("Unable to open fallback kernel");
			},
		Err(e) => panic!("Failed to open config file: {:?}", e),
		};
	// TODO: Load kernel from this file (ELF).
	// - Could just have the kernel be part of this image... but nah.


	loge!(conout, "> Spinning. TODO: Load kernel from system partition");
	
	loop {}
}


#[lang="eh_personality"]
fn eh_personality() -> ! {
	loop {}
}

#[no_mangle]
#[lang="panic_fmt"]
pub extern "C" fn rust_begin_unwind(msg: ::core::fmt::Arguments, _file: &'static str, _line: usize) -> ! {
	static mut NESTED: bool = false;
	unsafe {
		if NESTED {
			loop {}
		}
		NESTED = true;
		loge!(&*S_CONOUT, "PANIC: {}", msg);
	}
	loop {}
}

#[no_mangle]
pub extern "C" fn memcpy(dst: *mut u8, src: *const u8, count: usize) {
	unsafe {
		asm!("rep movsb" : : "{rcx}" (count), "{rdi}" (dst), "{rsi}" (src) : "rcx", "rsi", "rdi" : "volatile");
	}
}
#[no_mangle]
pub extern "C" fn memset(dst: *mut u8, val: u8, count: usize) {
	unsafe {
		asm!("rep stosb" : : "{rcx}" (count), "{rdi}" (dst), "{al}" (val) : "rcx", "rdi" : "volatile");
	}
}
#[no_mangle]
pub extern "C" fn memcmp(dst: *mut u8, src: *const u8, count: usize) -> isize {
	unsafe {
		let rv: isize;
		asm!("repnz cmpsb ; movq $$0, $0 ; ja 1f; jb 2f; jmp 3f; 1: inc $0 ; jmp 3f; 2: dec $0; 3:" : "=r" (rv) : "{rcx}" (count), "{rdi}" (dst), "{rsi}" (src) : "rcx", "rsi", "rdi" : "volatile");
		rv
	}
}

