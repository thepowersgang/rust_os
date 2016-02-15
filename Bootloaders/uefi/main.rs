#![feature(lang_items)]
#![feature(asm)]
#![no_std]

mod efi;

macro_rules! loge {
	($l:expr, $($t:tt)*) => {{
		use ::core::fmt::Write;
		let mut logger = ::EfiLogger($l);
		let _ = write!(&mut logger, "[{}] ", module_path!());
		let _ = write!(&mut logger, $($t)*); 
	}};
}


struct EfiLogger<'a>(&'a efi::SimpleTextOutputInterface);
impl<'a> ::core::fmt::Write for EfiLogger<'a> {
	fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
		for c in s.chars() {
			let mut b = [0, 0, 0];
			let c = c as u32;
			if c <= 0xD7FF {
				b[0] = c as u16;
			}
			else if c < 0xE000 {
				loop {}
			}
			else if c < 0x10000 {
				b[0] = c as u16;
			}
			else {
				let c2 = c - 0x10000;
				let (hi,lo) = (c2 >> 10, c2 & 0x3FF);
				// Surrogate time!
				b[0] = 0xD800 + hi as u16;
				b[1] = 0xDC00 + lo as u16;
			}
			self.0.output_string( b.as_ptr() );
		}
		Ok( () )
	}
}
impl<'a> Drop for EfiLogger<'a> {
	fn drop(&mut self) {
		self.0.output_string( [b'\r' as u16, b'\n' as u16, 0].as_ptr() );
	}
}


#[no_mangle]
pub extern "win64" fn efi_main(image_handle: efi::Handle, system_table: &efi::SystemTable) -> efi::Status
{
	// SAFE: Assuming that the system table data is valid
	let conout = system_table.con_out();
	loge!(conout, "efi_main(image_handle={:?}, system_table={:p})", image_handle, system_table);
	loge!(conout, "- Firmware Version {:x}", system_table.firmware_revision);
	//loge!(conout, "- Firmware Version {:#x} by '{}'", system_table.firmware_revision, system_table.firmware_vendor());


	loop {}
	0
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

