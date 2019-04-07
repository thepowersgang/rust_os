// UEFI Boot-loader
//
//
#![feature(asm)]
#![feature(proc_macro_hygiene)]	// utf16_literal
#![feature(panic_info_message)]
#![no_std] 

use uefi::boot_services::protocols;
use core::mem::size_of;

#[macro_use]
extern crate uefi;
extern crate utf16_literal;

macro_rules! log {
	($($v:tt)*) => { loge!( ::get_conout(), $($v)*) };
}
#[path="../_common/elf.rs"]
mod elf;

#[path="../uefi_proto.rs"]
mod kernel_proto;

static PATH_CONFIG: &'static [u16] = ::utf16_literal::utf16!("Tifflin\\boot.cfg\0");
static PATH_FALLBACK_KERNEL: &'static [u16] = ::utf16_literal::utf16!("Tifflin\\kernel-amd4.bin\0");

// Globals used for panic handling and loging
static mut S_CONOUT: *const ::uefi::SimpleTextOutputInterface = 1 as *const _;
static mut S_BOOT_SERVICES: *const ::uefi::boot_services::BootServices = 0 as *const _;
static mut S_IMAGE_HANDLE: ::uefi::Handle = 0 as *mut _;

struct Configuration<'bs>
{
	kernel: ::uefi::borrow::Cow<'bs, 'static, ::uefi::CStr16>,
	//commandline: ::uefi::borrow::Cow<'bs, 'static, str>,
}
impl<'bs> Configuration<'bs>
{
	fn from_file(_bs: &'bs ::uefi::boot_services::BootServices, sys_vol: &protocols::File, filename: &::uefi::CStr16) -> Result<Configuration<'bs>, ::uefi::Status>
	{
		match sys_vol.open_read(filename)
		{
		Ok(_cfg_file) => {
			panic!("TODO: Read config file (allocating strings with `bs`)");
			},
		Err(::uefi::status::NOT_FOUND) => {
			Ok(Configuration {
				kernel: ::uefi::CStr16::from_slice(PATH_FALLBACK_KERNEL).into(),
				//commandline: "".into(),
				})
			},
		Err(e) => Err(e),
		}
	}
}

pub fn get_conout() -> &'static ::uefi::SimpleTextOutputInterface {
	// SAFE: Immutable after efi_main starts running
	unsafe { &*S_CONOUT }
}

#[no_mangle]
pub extern "win64" fn efi_main(image_handle: ::uefi::Handle, system_table: &::uefi::SystemTable) -> ::uefi::Status
{
	let conout = system_table.con_out();

	// Set up globals for panic! and log!
	// SAFE: Single-threaded context
	unsafe {
		S_CONOUT = conout;
		S_IMAGE_HANDLE = image_handle;
		S_BOOT_SERVICES = system_table.boot_services;
	}


	loge!(conout, "efi_main(image_handle={:?}, system_table={:p}) - {:p}", image_handle, system_table, { #[link_section=".text"] static S_MARKER: () = (); &S_MARKER });
	//loge!(conout, "- RSP: {:p}", unsafe { let v: u64; asm!("mov %rsp, $0" : "=r" (v)); v } as usize as *const ());
	loge!(conout, "- Firmware Version {:#x} by '{}'", system_table.firmware_revision, system_table.firmware_vendor());
	loge!(conout, "- Boot Services @ {:p}, Runtime Services @ {:p}",
		system_table.boot_services, system_table.runtime_services);
	
	{
		let boot_services = system_table.boot_services;

		// Obtain the "LoadedImage" representing the bootloader, from which we get the boot volume
		let image_proto: &protocols::LoadedImage = boot_services.handle_protocol(&image_handle).expect("image_handle - LoadedImage");
		if image_proto.file_path.type_code() != (4,4) {
			panic!("Loader wans't loaded from a filesystem - type_code = {:?}", image_proto.file_path.type_code());
		}
		let system_volume_fs: &protocols::SimpleFileSystem = boot_services.handle_protocol(&image_proto.device_handle).expect("image_proto - FileProtocol");
		// - Get the root of this volume and load the bootloader configuration file from it
		let system_volume_root = system_volume_fs.open_volume().expect("system_volume_fs - File");
		// NOTE: This function will return Ok(Default::default()) if the file can't be found
		let config = match Configuration::from_file(boot_services, &system_volume_root, PATH_CONFIG.into())
			{
			Ok(c) => c,
			Err(e) => panic!("Failed to load config file: {:?}", e),
			};
		// - Load the kernel.
		let entrypoint = load_kernel_file(boot_services, &system_volume_root, &config.kernel).expect("Unable to load kernel");

		// TODO: Set a sane video mode
		
		// Save memory map
		let (map_key, map) = {
			let mut map_size = 0;
			let mut map_key = 0;
			let mut ent_size = 0;
			let mut ent_ver = 0;
			match unsafe { (boot_services.get_memory_map)(&mut map_size, ::core::ptr::null_mut(), &mut map_key, &mut ent_size, &mut ent_ver) }
			{
			::uefi::status::SUCCESS => {},
			::uefi::status::BUFFER_TOO_SMALL => {},
			e => panic!("get_memory_map - {:?}", e),
			}

			assert_eq!( ent_size, size_of::<uefi::boot_services::MemoryDescriptor>() );
			let mut map;
			loop
			{
				map = boot_services.allocate_pool_vec( uefi::boot_services::MemoryType::LoaderData, map_size / ent_size ).unwrap();
				match unsafe { (boot_services.get_memory_map)(&mut map_size, map.as_mut_ptr(), &mut map_key, &mut ent_size, &mut ent_ver) }
				{
				::uefi::status::SUCCESS => break,
				::uefi::status::BUFFER_TOO_SMALL => continue,
				e => panic!("get_memory_map 2 - {:?}", e),
				}
			}
			unsafe {
				map.set_len( map_size / ent_size );
			}

			(map_key, map)
			};
		loge!(conout, "- Exiting boot services");
		//let runtime_services = system_table_ptr.exit_boot_services().ok().expect("exit_boot_services");
		// SAFE: Weeelll...
		unsafe { 
			(boot_services.exit_boot_services)(image_handle, map_key).expect("exit_boot_services");
		}

		let boot_info = kernel_proto::Info {
			runtime_services: system_table.runtime_services as *const _ as *const (),
			
			// TODO: Get from the configuration
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

type Entrypoint = extern "cdecl" fn(usize, *const kernel_proto::Info)->!;
fn load_kernel_file(boot_services: &::uefi::boot_services::BootServices, sys_vol: &protocols::File, filename: &::uefi::CStr16) -> Result<Entrypoint, ::uefi::Status>
{
	let mut kernel_file = match sys_vol.open_read(filename)
		{
		Ok(k) => k,
		Err(e) => panic!("Failed to open kernel '{}' - {:?}", filename, e),
		};
	// Load kernel from this file (ELF).
	let elf_hdr = {
		let mut hdr = elf::ElfHeader::default();
		// SAFE: Converts to POD for read
		kernel_file.read( unsafe { ::core::slice::from_raw_parts_mut( &mut hdr as *mut _ as *mut u8, size_of::<elf::ElfHeader>() ) } ).expect("ElfHeader read");
		hdr
		};
	elf_hdr.check_header();
	for i in 0 .. elf_hdr.e_phnum
	{
		let mut ent = elf::PhEnt::default();
		kernel_file.set_position(elf_hdr.e_phoff as u64 + (i as usize * size_of::<elf::PhEnt>()) as u64 ).expect("PhEnt seek");
		// SAFE: Converts to POD for read
		kernel_file.read( unsafe { ::core::slice::from_raw_parts_mut( &mut ent as *mut _ as *mut u8, size_of::<elf::PhEnt>() ) } ).expect("PhEnt read");
		
		if ent.p_type == 1
		{
			log!("- {:#x}+{:#x} loads +{:#x}+{:#x}",
				ent.p_paddr, ent.p_memsz,
				ent.p_offset, ent.p_filesz
				);
			
			let mut addr = ent.p_paddr as u64;
			// SAFE: Correct call to FFI
			unsafe { (boot_services.allocate_pages)(
				::uefi::boot_services::AllocateType::Address,
				::uefi::boot_services::MemoryType::LoaderData,
				(ent.p_memsz + 0xFFF) as usize / 0x1000,
				&mut addr
				)
				.expect("Allocating pages for program segment")
				;}
			
			// SAFE: This memory has just been allocated by the above
			let data_slice = unsafe { ::core::slice::from_raw_parts_mut(ent.p_paddr as usize as *mut u8, ent.p_memsz as usize) };
			kernel_file.set_position(ent.p_offset as u64).expect("seek segment");
			kernel_file.read( &mut data_slice[.. ent.p_filesz as usize] ).expect("read segment");
			for b in &mut data_slice[ent.p_filesz as usize .. ent.p_memsz as usize] {
				*b = 0;
			}
		}
	}
	// SAFE: Assuming that the executable is sane, and that it follows the correct calling convention
	Ok(unsafe { ::core::mem::transmute(elf_hdr.e_entry as usize) })
}


#[panic_handler]
fn handle_panic(info: &::core::panic::PanicInfo) -> ! {
	static mut NESTED: bool = false;
	unsafe {
		if NESTED {
			loop {}
		}
		NESTED = true;
		if let Some(m) = info.message() {
			loge!(&*S_CONOUT, "PANIC: {}", m);
		}
		else if let Some(m) = info.payload().downcast_ref::<&str>() {
			loge!(&*S_CONOUT, "PANIC: {}", m);
		}
		else {
			loge!(&*S_CONOUT, "PANIC: ?");
		}

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

