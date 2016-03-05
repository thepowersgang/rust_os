//
//
//
#![no_std]
#![feature(no_std,lang_items)]
#![feature(core_str_ext,core_slice_ext)]

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

include!{"_common/elf.rs"}


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
