// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/pci.rs
//! Legacy PCI bus access
use crate::lib::mem::aref::Aref;
use crate::lib::lazy_static::LazyStatic;

pub(super) fn init()
{
	// Register the x86 legacy interface as a bus via the PCI manager
	static S_BUILTIN_INTERFACE: LazyStatic<Aref<BuiltinInterface>> = LazyStatic::new();
	crate::hw::bus_pci::register_bus( S_BUILTIN_INTERFACE.prep(|| Aref::new(BuiltinInterface)).borrow() );
}

static S_PCI_LOCK: crate::sync::Spinlock<PCICfgSpace> = crate::sync::Spinlock::new(PCICfgSpace);

struct PCICfgSpace;
impl PCICfgSpace
{
	fn read(&mut self, addr: u32) -> u32 {
		// SAFE: (from accessing the wrong place)
		unsafe {
			crate::arch::x86_io::outl(0xCF8, 0x80000000 | addr);
			crate::arch::x86_io::inl(0xCFC)
		}
	}
	fn write(&mut self, addr: u32, val: u32) {
		// SAFE: (from accessing the wrong place)
		unsafe {
			crate::arch::x86_io::outl(0xCF8, 0x80000000 | addr);
			crate::arch::x86_io::outl(0xCFC, val)
		}
	}
}

struct BuiltinInterface;
impl BuiltinInterface
{
	/// Translate address
	fn get_addr(bus_addr: u16, word_idx: u8) -> u32
	{
		((bus_addr as u32) << 8) | ((word_idx as u32) << 2)
	}
}
impl crate::hw::bus_pci::PciInterface for BuiltinInterface
{
	fn read_word(&self, bus_addr: u16, word_idx: u8) -> u32 {
		let addr = Self::get_addr(bus_addr, word_idx);
		//log_trace!("read_word(bus_addr={:x},idx={}) addr={:#x}", bus_addr, wordidx, addr);
		S_PCI_LOCK.lock().read(addr)
	}
	unsafe fn write_word(&self, bus_addr: u16, word_idx: u8, val: u32) {
		let addr = Self::get_addr(bus_addr, word_idx);
		//log_trace!("read_word(bus_addr={:x},idx={}) addr={:#x}", bus_addr, wordidx, addr);
		S_PCI_LOCK.lock().write(addr, val)
	}
	unsafe fn get_mask(&self, bus_addr: u16, word_idx: u8, in_mask: u32) -> (u32, u32) {
		let addr = Self::get_addr(bus_addr, word_idx);
		let mut lh = S_PCI_LOCK.lock();
		let old_value = lh.read(addr);
		lh.write(addr, in_mask);
		let new_value = lh.read(addr);
		lh.write(addr, old_value);
		(old_value, new_value)
	}

	/// Returns the IRQ number (suitable for the ::irq module) for the specified pin
	#[cfg(false_)]
	fn get_isr_for_pin(pin: u8) -> u32
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
	#[cfg(false_)]
	pub fn enable_interrupt_pin(_pin: u8)
	{
		// Poke the IOAPIC and LAPIC to allow that interrupt through
		unimplemented!();
	}
}

// vim: ft=rust

