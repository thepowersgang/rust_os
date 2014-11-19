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

// vim: ft=rust

