

pub fn write(addr: u32, value: u32) {
	todo!("PCI write {:#x} v {:#x}", addr, value);
}
pub fn read(addr: u32) -> u32 {
	log_trace!("TODO: PCI read {:#x}", addr);
	!0
}

