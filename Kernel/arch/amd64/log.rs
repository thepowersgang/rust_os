// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/log.rs
// - RS232 logging output
use _common::*;

use super::x86_io;

pub fn puts(text: &str)
{
	for c in text.bytes()
	{
		putc(c);
	}
}
pub fn puth(val: u64)
{
	puts("0");
	let nibbles = {
		let mut v = 1u64;
		while (val >> v*4) > 0 && v < 64/4 { v += 1 }
		v
		};
	//let nibbles = 16u;
	puts("x");
	for i in ::core::iter::range(0, nibbles)
	{
		let nibble : u8 = ((val >> (nibbles-i-1)*4) & 15) as u8;
		putc( if nibble <= 9 { '0' as u8 + nibble } else { 'a' as u8 + nibble-10 } );
	}
}
fn putc(c: u8)
{
	unsafe
	{
		while (x86_io::inb(0x3F8+5) & 0x20) == 0
		{
		}
		x86_io::outb(0x3F8, c as u8);
		x86_io::outb(0xe9, c as u8);
	}
}

// vim: ft=rust

