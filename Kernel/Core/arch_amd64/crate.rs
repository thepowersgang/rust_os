//
//
//
#![feature(asm)]
#![crate_type="lib"]
//#![no_std]

//extern crate core;
//use core::raw::Slice;

pub fn puts(text: &str)
{
	for c in text.bytes()
	{
		putc(c);
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

