// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/pci.rs
//! PCI bus access

static S_PCI_LOCK: ::sync::Spinlock<()> = ::sync::Spinlock::new( () );

/// Read a word from a pre-calculated PCI address
pub fn read(addr: u32) -> u32
{
	let _lh = S_PCI_LOCK.lock();
	unsafe {
		::arch::x86_io::outl(0xCF8, 0x80000000 | addr);
		::arch::x86_io::inl(0xCFC)
	}
}

/// Write a word to a pre-calculated PCI address
pub fn write(addr: u32, val: u32)
{
	let _lh = S_PCI_LOCK.lock();
	unsafe {
		::arch::x86_io::outl(0xCF8, 0x80000000 | addr);
		::arch::x86_io::outl(0xCFC, val)
	}
}

/// Returns the IRQ number (suitable for the ::irq module) for the specified pin
pub fn get_isr_for_pin(pin: u8) -> u32
{
	match pin
	{
	0 => 8,
	1 => 9,
	2 => 10,
	3 => 11,
	_ => panic!("Unknown PCI interrupt pin {}", pin),
	}
}

/// Enables the specified pin
pub fn enable_interrupt_pin(_pin: u8)
{
	// Poke the IOAPIC and LAPIC to allow that interrupt through
	unimplemented!();
}

// vim: ft=rust

