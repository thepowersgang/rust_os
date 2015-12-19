// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/x86_io.rs
//! Support for x86's IO bus

/// Read a single byte
#[inline]
pub unsafe fn inb(port: u16) -> u8 {
	let ret : u8;
	asm!("inb $1, $0" : "={ax}"(ret) : "{dx}N"(port) : : "volatile");
	return ret;
}
/// Write a single byte
#[inline]
pub unsafe fn outb(port: u16, val: u8) {
	asm!("outb $1, $0" : : "{dx}N"(port), "{al}"(val) : : "volatile");
}

/// Read a 16-bit word
#[inline]
pub unsafe fn inw(port: u16) -> u16 {
	let ret : u16;
	asm!("inw $1, $0" : "={ax}"(ret) : "{dx}N"(port) : : "volatile");
	return ret;
}
/// Write a 16-bit word
#[inline]
pub unsafe fn outw(port: u16, val: u16) {
	asm!("outw %ax, $0" : : "{dx}N"(port), "{ax}"(val) : : "volatile");
}

/// Read a 32-bit long/double-word
#[inline]
pub unsafe fn inl(port: u16) -> u32 {
	let ret : u32;
	asm!("inl $1, $0" : "={eax}"(ret) : "{dx}N"(port) : : "volatile");
	return ret;
}
/// Write a 32-bit long/double-word
#[inline]
pub unsafe fn outl(port: u16, val: u32) {
	asm!("outl %eax, $0" : : "{dx}N"(port), "{eax}"(val) : : "volatile");
}

// vim: ft=rust

