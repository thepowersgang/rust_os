//
// Interface to architecture-specific code (usually assembly)
//

extern "C" {
	fn ext_puts(text: &str);
}

pub fn puts(text: &str) {
	unsafe { ext_puts(text); }
}
// vim: ft=rust
