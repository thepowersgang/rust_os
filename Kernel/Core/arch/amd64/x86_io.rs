// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/x86_io.rs
//! Support for x86's IO bus

/// Read a single byte
#[inline]
pub unsafe fn inb(port: u16) -> u8 {
	let ret : u8;
	::core::arch::asm!("in al, dx", out("al") ret, in("dx") port, options(preserves_flags, nomem, nostack));
	return ret;
}
/// Write a single byte
#[inline]
pub unsafe fn outb(port: u16, val: u8) {
	::core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(preserves_flags, nomem, nostack));
}

/// Read a 16-bit word
#[inline]
pub unsafe fn inw(port: u16) -> u16 {
	let ret : u16;
	::core::arch::asm!("in ax, dx", out("ax") ret, in("dx") port, options(preserves_flags, nomem, nostack));
	return ret;
}
/// Write a 16-bit word
#[inline]
pub unsafe fn outw(port: u16, val: u16) {
	::core::arch::asm!("out dx, ax", in("dx") port, in("ax") val, options(preserves_flags, nomem, nostack));
}

/// Read a 32-bit long/double-word
#[inline]
pub unsafe fn inl(port: u16) -> u32 {
	let ret : u32;
	::core::arch::asm!("in eax, dx", out("eax") ret, in("dx") port, options(preserves_flags, nomem, nostack));
	return ret;
}
/// Write a 32-bit long/double-word
#[inline]
pub unsafe fn outl(port: u16, val: u32) {
	::core::arch::asm!("out dx, eax", in("dx") port, in("eax") val, options(preserves_flags, nomem, nostack));
}

// vim: ft=rust

