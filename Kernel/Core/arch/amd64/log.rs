// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/log.rs
//! RS232 logging output
#[allow(unused_imports)]
use crate::prelude::*;

use super::x86_io;

#[inline(never)]
/// Print a string to the logging output
pub fn puts(text: &str)
{
	for c in text.bytes()
	{
		putc(c);
	}
}
/// Print a hexadecimal value to the logging output
pub fn puth(val: u64) {
	let nibbles = {
		let mut v = 1;
		while (val >> v*4) > 0 && v < 64/4 { v += 1 }
		v
		};
	puts("0x");
	puth_digits(val, nibbles);
}
pub fn puth64(val: u64) {
	puth_digits(val, 64/4)
}
pub fn puth_digits(val: u64, nibbles: usize) {
	for i in 0 .. nibbles
	{
		let nibble : u8 = ((val >> (nibbles-i-1)*4) & 15) as u8;
		putc( if nibble <= 9 { '0' as u8 + nibble } else { 'a' as u8 + nibble-10 } );
	}
}
/// Print a single character to the logging output
fn putc(c: u8)
{
	// SAFE: Racy I guess... but doesn't cause memory unsafety (and worst is lost chars)
	unsafe {
		while (x86_io::inb(0x3F8+5) & 0x20) == 0
		{
		}
		x86_io::outb(0x3F8, c as u8);
		x86_io::outb(0xe9, c as u8);
	}
}

// vim: ft=rust

