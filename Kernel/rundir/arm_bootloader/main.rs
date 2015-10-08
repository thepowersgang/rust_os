//
//
//
#![no_std]
#![feature(no_std,lang_items)]
#![feature(core_str_ext)]


/// Stub logging macro
macro_rules! log{
	($($v:tt)*) => {{
		use core::fmt::Write;
		let mut lh = ::Logger;
		let _ = write!(lh, "[loader log] ");
		let _ = write!(lh, $($v)*);
		let _ = write!(lh, "\n");
		}};
}

#[no_mangle]
pub extern "C" fn elf_get_size(file_base: *const u8) -> u32
{
	log!("elf_get_size(file_base={:p})", file_base);
	0
}

#[no_mangle]
pub extern "C" fn elf_load_segments(file_base: *const u8, output_base: *const u8) -> u32
{
	log!("elf_load_segments(file_base={:p}, output_base={:p})", file_base, output_base);
	0
}

#[no_mangle]
pub extern "C" fn elf_load_symbols(file_base: *const u8, output_base: *const u8) -> u32
{
	log!("elf_load_symbols(file_base={:p}, output_base={:p})", file_base, output_base);
	0
}



//
//
//

#[lang="eh_personality"]
fn eh_personality() -> ! {
	loop {}
}
#[lang="panic_fmt"]
fn panic_fmt() -> ! {
	loop {}
}


extern "C" {
	fn puts(_: *const u8, _: u32);
}
struct Logger;
impl ::core::fmt::Write for Logger {
	fn write_str(&mut self, s: &str) -> ::core::fmt::Result
	{
		// SAFE: Single-threaded
		unsafe {
			puts(s.as_ptr(), s.len() as u32);
		}
		Ok( () )
	}
}
