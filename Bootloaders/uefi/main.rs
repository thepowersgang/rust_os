//
//
//
#![feature(lang_items)]
#![feature(asm)]
#![no_std] 
//#![crate_type="lib"]

use uefi::boot_services::protocols;
use core::mem::size_of;

#[macro_use]
extern crate uefi;

macro_rules! log {
	($($v:tt)*) => { loge!( ::get_conout(), $($v)*) };
}
#[path="../_common/elf.rs"]
mod elf;

#[path="../uefi_proto.rs"]
mod kernel_proto;

// TODO: Write a procedural macro that creates a UCS2 string literal
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
static mut S_BOOT_SERVICES: *const ::uefi::boot_services::BootServices = 0 as *const _;
static mut S_IMAGE_HANDLE: ::uefi::Handle = 0 as *mut _;

pub fn get_conout() -> &'static ::uefi::SimpleTextOutputInterface {
	// SAFE: Immutable after efi_main starts running
	unsafe { &*S_CONOUT }
}

#[no_mangle]
pub extern "win64" fn efi_main(image_handle: ::uefi::Handle, system_table: &::uefi::SystemTable) -> ::uefi::Status
{
	let conout = system_table.con_out();
	// SAFE: Single-threaded context
	unsafe {
		S_CONOUT = conout;
		S_IMAGE_HANDLE = image_handle;
		S_BOOT_SERVICES = system_table.boot_services;
	}
	loge!(conout, "efi_main(image_handle={:?}, system_table={:p}) - {:p}", image_handle, system_table, &S_MARKER);
	//let sp = unsafe { let v: u64; asm!("mov %rsp, $0" : "=r" (v)); v };
	//loge!(conout, "- RSP: {:p}", sp as usize as *const ());
	loge!(conout, "- Firmware Version {:#x} by '{}'", system_table.firmware_revision, system_table.firmware_vendor());
	loge!(conout, "- Boot Services @ {:p}, Runtime Services @ {:p}",
		system_table.boot_services, system_table.runtime_services);
	
	{
		let boot_services = system_table.boot_services;
		//let image_dev: &protocols::LoadedImageDevicePath = boot_services.handle_protocol(&image_handle).expect("image_handle - LoadedImageDevicePath");
		//loge!(conout, "- image_dev = {:?}", image_dev);
		let image_proto: &protocols::LoadedImage = boot_services.handle_protocol(&image_handle).expect("image_handle - LoadedImage");
		//loge!(conout, "- image_proto.file_path={:?}", image_proto.file_path);
		
		if image_proto.file_path.type_code() != (4,4) {
			panic!("Loader wans't loaded from a filesystem");
		}

		let system_volume_fs: &protocols::SimpleFileSystem = boot_services.handle_protocol(&image_proto.device_handle).expect("image_proto - FileProtocol");
		let system_volume_root = system_volume_fs.open_volume().expect("system_volume_fs - File");
		
		let mut kernel_file = match system_volume_root.open_read(PATH_CONFIG)
			{
			Ok(cfg) => {
				panic!("TODO: Read config file");
				},
			Err(::uefi::status::NOT_FOUND) => {
				// If the config file couldn't be located, open a hard-coded fallback kernel path
				system_volume_root.open_read(PATH_FALLBACK_KERNEL).expect("Unable to open fallback kernel")
				},
			Err(e) => panic!("Failed to open config file: {:?}", e),
			};
		// Load kernel from this file (ELF).
		let elf_hdr = {
			let mut hdr = elf::ElfHeader::default();
			kernel_file.read( unsafe { ::core::slice::from_raw_parts_mut( &mut hdr as *mut _ as *mut u8, size_of::<elf::ElfHeader>() ) } ).expect("ElfHeader read");
			hdr
			};
		elf_hdr.check_header();
		for i in 0 .. elf_hdr.e_phnum
		{
			let mut ent = elf::PhEnt::default();
			kernel_file.set_position(elf_hdr.e_phoff as u64 + (i as usize * size_of::<elf::PhEnt>()) as u64 ).expect("PhEnt seek");
			kernel_file.read( unsafe { ::core::slice::from_raw_parts_mut( &mut ent as *mut _ as *mut u8, size_of::<elf::PhEnt>() ) } ).expect("PhEnt read");
			
			if ent.p_type == 1
			{
				loge!(conout, "- {:#x}+{:#x} loads +{:#x}+{:#x}",
					ent.p_paddr, ent.p_memsz,
					ent.p_offset, ent.p_filesz
					);
				
				let mut addr = ent.p_paddr as u64;
				(boot_services.allocate_pages)(
					::uefi::boot_services::AllocateType::Address,
					::uefi::boot_services::MemoryType::LoaderData,
					(ent.p_memsz + 0xFFF) as usize / 0x1000,
					&mut addr
					)
					.err_or( () )	// uefi::Status -> Result<(), Status>
					.expect("allocate_pages")
					;
				
				// SAFE: This memory has just been allocated by the above
				let data_slice = unsafe { ::core::slice::from_raw_parts_mut(ent.p_paddr as usize as *mut u8, ent.p_memsz as usize) };
				kernel_file.set_position(ent.p_offset as u64).expect("seek segment");
				kernel_file.read( &mut data_slice[.. ent.p_filesz as usize] ).expect("read segment");
				for b in &mut data_slice[ent.p_filesz as usize .. ent.p_memsz as usize] {
					*b = 0;
				}
			}
		}
		// SAFE: Assuming that the executable is sane
		let entrypoint: extern "cdecl" fn(usize, *const kernel_proto::Info)->! = unsafe { ::core::mem::transmute(elf_hdr.e_entry as usize) };

		// TODO: Set a sane video mode
		
		// Save memory map
		let (map_key, map) = {
			let mut map_size = 0;
			let mut map_key = 0;
			let mut ent_size = 0;
			let mut ent_ver = 0;
			match (boot_services.get_memory_map)(&mut map_size, ::core::ptr::null_mut(), &mut map_key, &mut ent_size, &mut ent_ver)
			{
			::uefi::status::SUCCESS => {},
			::uefi::status::BUFFER_TOO_SMALL => {},
			e => panic!("get_memory_map - {:?}", e),
			}

			assert_eq!( ent_size, size_of::<uefi::boot_services::MemoryDescriptor>() );
			let mut map = boot_services.allocate_pool_vec( uefi::boot_services::MemoryType::LoaderData, map_size / ent_size ).unwrap();
			(boot_services.get_memory_map)(&mut map_size, map.as_mut_ptr(), &mut map_key, &mut ent_size, &mut ent_ver).err_or( () ).expect("get_memory_map 2");
			unsafe {
				map.set_len( map_size / ent_size );
			}

			(map_key, map)
			};
		loge!(conout, "- Exiting boot services");
		(boot_services.exit_boot_services)(image_handle, map_key).err_or( () ).expect("exit_boot_services");

		let boot_info = kernel_proto::Info {
			runtime_services: system_table.runtime_services as *const _ as *const (),
			
			cmdline_ptr: 1 as *const u8,
			cmdline_len: 0,
			
			map_addr: map.as_ptr() as usize as u64,
			map_entnum: map.len() as u32,
			map_entsz: size_of::<uefi::boot_services::MemoryDescriptor>() as u32,
			};
		
		
		// - Execute kernel (passing a magic value and general boot information)
		entrypoint(0x71FF0EF1, &boot_info);
	}
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

		((*S_BOOT_SERVICES).exit)(S_IMAGE_HANDLE, ::uefi::status::NOT_FOUND, 0, ::core::ptr::null());
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

