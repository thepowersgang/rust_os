// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/arch/armv8/mod.rs
// - ARMv8 (AArch64) interface
pub mod memory;
pub mod pci;
pub mod sync;
pub mod threads;
pub mod boot;
pub mod interrupts;

#[path="../armv7/fdt.rs"]
mod fdt;

pub fn print_backtrace() {
	let mut fp: *const FrameEntry;
	// SAFE: Just loads the frame pointer
	unsafe { asm!("mov $0, fp" : "=r"(fp)); }

	#[repr(C)]
	struct FrameEntry {
		next: *const FrameEntry,
		ret_addr: usize,
	}
	puts("Backtrace:");
	while ! fp.is_null()
	{
		//if ! ::memory::virt::is_reserved(data_ptr) {
		//	break;
		//}
		// SAFE: Checked by above
		let data = unsafe { &*fp };
		puts(" -> "); puth(data.ret_addr as u64);
		if let Some( (name,ofs) ) = ::symbols::get_symbol_for_addr(data.ret_addr) {
			puts("("); puts(name); puts("+"); puth(ofs as u64); puts(")");
		}
		fp = data.next;
	}
	puts("\n");
}

pub fn cur_timestamp() -> u64 {
	0
}

extern "C" {
	pub fn drop_to_user(entry: usize, stack: usize, args_len: usize) -> !;
}


pub mod x86_io {
	pub unsafe fn inb(_p: u16) -> u8 { panic!("calling inb on ARM") }
	pub unsafe fn inw(_p: u16) -> u16 { panic!("calling inw on ARM") }
	pub unsafe fn inl(_p: u16) -> u32 { panic!("calling inl on ARM") }
	pub unsafe fn outb(_p: u16, _v: u8) {}
	pub unsafe fn outw(_p: u16, _v: u16) {}
	pub unsafe fn outl(_p: u16, _v: u32) {}
}


fn putb(b: u8) {
	// SAFE: Access should be correct, and no race is possible
	unsafe {
		// - First HWMap page is the UART
		let uart = memory::addresses::HARDWARE_BASE as *mut u8;
		::core::intrinsics::volatile_store( uart.offset(0), b );
	}
}
#[inline(never)]
#[no_mangle]
pub fn puts(s: &str) {
	for b in s.bytes() {
		putb(b);
	}
}
#[inline(never)]
#[no_mangle]
pub fn puth(v: u64) {
	putb(b'0');
	putb(b'x');
	if v == 0 {
		putb(b'0');
	}
	else {
		for i in (0 .. 16).rev() {
			if v >> (i * 4) > 0 {
				let n = ((v >> (i * 4)) & 0xF) as u8;
				if n < 10 {
					putb( b'0' + n );
				}
				else {
					putb( b'a' + n - 10 );
				}
			}
		}
	}
}

