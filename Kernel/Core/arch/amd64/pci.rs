//
//
//

#[allow(non_upper_case_globals)]
static s_pci_lock: ::sync::Spinlock<()> = spinlock_init!(());

pub fn read(addr: u32) -> u32
{
	let _lh = s_pci_lock.lock();
	unsafe {
		::arch::x86_io::outl(0xCF8, 0x80000000 | addr);
		::arch::x86_io::inl(0xCFC)
	}
}

pub fn write(addr: u32, val: u32)
{
	let _lh = s_pci_lock.lock();
	unsafe {
		::arch::x86_io::outl(0xCF8, 0x80000000 | addr);
		::arch::x86_io::outl(0xCFC, val)
	}
}

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

pub fn enable_interrupt_pin(pin: u8)
{
	// Poke the IOAPIC and LAPIC to allow that interrupt through
}

// vim: ft=rust

