//
//
//
#![no_std]
#![feature(lang_items)]

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

#[path="../_common/elf.rs"]
mod elf;

#[no_mangle]
pub extern "C" fn elf_get_size(file_base: &elf::ElfFile) -> u32
{
	elf::elf_get_size(file_base)
}
/// Returns program entry point
#[no_mangle]
pub extern "C" fn elf_load_segments(file_base: &elf::ElfFile, output_base: *mut u8) -> u32
{
	elf::elf_load_segments(file_base, output_base)
}
#[no_mangle]
/// Returns size of data written to output_base
pub extern "C" fn elf_load_symbols(file_base: &elf::ElfFile, output: &mut elf::SymbolInfo) -> u32
{
	elf::elf_load_symbols(file_base, output)
}


//
//
//

#[lang="eh_personality"]
fn eh_personality() -> ! {
	puts("UNWIND");
	loop {}
}
#[panic_handler]
fn panic_fmt(_: &::core::panic::PanicInfo) -> ! {
	puts("PANIC");
	loop {}
}


fn puts(s: &str) {
	extern "C" {
		fn puts(_: *const u8, _: u32);
	}
	// SAFE: Single-threaded
	unsafe {
		puts(s.as_ptr(), s.len() as u32);
	}
}
struct Logger;
impl ::core::fmt::Write for Logger {
	fn write_str(&mut self, s: &str) -> ::core::fmt::Result
	{
		puts(s);
		Ok( () )
	}
}
