#![feature(lang_items)]
#![feature(asm)]
#![no_std] 
//#![crate_type="obj"]

#[macro_use]
extern crate uefi;

macro_rules! log {
	($($v:tt)*) => { loge!( ::get_conout(), $($v)*) };
}
#[path="../_common/elf.rs"]
mod elf;

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
	let sp = unsafe { let v: u64; asm!("mov %rsp, $0" : "=r" (v)); v };
	loge!(conout, "- RSP: {:p}", sp as usize as *const ());
	loge!(conout, "- Firmware Version {:#x} by '{}'", system_table.firmware_revision, system_table.firmware_vendor());
	loge!(conout, "- Boot Services @ {:p}, Runtime Services @ {:p}",
		system_table.boot_services, system_table.runtime_services);

	loop {}
}


#[lang="eh_personality"]
fn eh_personality() -> ! {
	loop {}
}
#[lang="panic_fmt"]
fn panic_fmt() -> ! {
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

