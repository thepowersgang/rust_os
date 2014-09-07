//
//
//
#![feature(asm)]
#![crate_type="lib"]
#![no_std]
extern crate core;

use core::str::StrSlice;

pub mod float;
pub mod interrupts;

pub fn puts(text: &str)
{
	for c in text.bytes()
	{
		putc(c);
	}
}
pub fn puth(val: uint)
{
	puts("0");
	let nibbles = {
		let mut v = 1u;
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
		while (inb(0x3F8+5) & 0x20) == 0
		{
		}
		outb(0x3F8, c as u8);
	}
}

unsafe fn inb(port: u16) -> u8 {
	let ret : u8;
	asm!("inb %dx, %al" : "={ax}"(ret) : "{dx}"(port));
	return ret;
}
unsafe fn outb(port: u16, val: u8) {
	asm!("outb %al, %dx" : : "{dx}"(port), "{al}"(val));
}

// vim: ft=rust

