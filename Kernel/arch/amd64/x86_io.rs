// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/x86_io.rs
// - Support for x86's IO bus

pub unsafe fn inb(port: u16) -> u8 {
	let ret : u8;
	asm!("inb %dx, %al" : "={ax}"(ret) : "{dx}"(port));
	return ret;
}
pub unsafe fn outb(port: u16, val: u8) {
	asm!("outb %al, %dx" : : "{dx}"(port), "{al}"(val));
}

// vim: ft=rust

