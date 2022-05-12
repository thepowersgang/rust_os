//! PIT - Programmable Interval Timer
//! 
//! This is the legacy PC timer chip, simulated by the chipset on modern computers.

/// Disable the PIT, once the HPET is fully configured
pub(super) unsafe fn disable()
{
    crate::arch::x86_io::outb(0x43, 0<<7|3<<4|0);
    crate::arch::x86_io::outb(0x43, 1<<7|3<<4|0);
    crate::arch::x86_io::outb(0x43, 2<<7|3<<4|0);
    crate::arch::x86_io::outb(0x43, 3<<7|3<<4|0);
}