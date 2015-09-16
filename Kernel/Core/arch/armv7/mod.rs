
pub mod memory;

pub mod sync;

pub mod interrupts;

pub mod boot;

pub mod pci;

pub mod threads;


pub unsafe fn drop_to_user(entry: usize, stack: usize, args_len: usize) -> !
{
	loop {}
}


pub fn puts(s: &str) {
}
pub fn puth(v: u64) {
}

pub fn cur_timestamp() -> u64 {
	0
}

pub fn print_backtrace() {
}

pub mod x86_io {
	pub unsafe fn inb(_p: u16) -> u8 { panic!("calling inb on ARM") }
	pub unsafe fn inw(_p: u16) -> u16 { panic!("calling inw on ARM") }
	pub unsafe fn inl(_p: u16) -> u32 { panic!("calling inl on ARM") }
	pub unsafe fn outb(_p: u16, _v: u8) {}
	pub unsafe fn outw(_p: u16, _v: u16) {}
	pub unsafe fn outl(_p: u16, _v: u32) {}
}


#[repr(C)]
pub struct ulldiv_t {
	quo: u64,
	rem: u64,
}
#[no_mangle]
pub extern fn __aeabi_uldivmod(mut n: u64, mut d: u64) -> ulldiv_t {
	let mut ret = 0;
	let mut add = 1;
	while n / 2 >= d && add != 0 { d <<= 1; add <<= 1; }
	while add > 0 { if n >= d { ret += add; n -= d; } add  >>= 1; d >>= 1; }

	ulldiv_t { quo: ret, rem: n, }
}
#[no_mangle]
pub extern fn __umoddi3(n: u64, d: u64) -> u64 {
	__aeabi_uldivmod(n, d).rem
}

#[repr(C)]
pub struct uidiv_t {
	quo: u32,
	rem: u32,
}
#[no_mangle]
pub extern fn __aeabi_uidivmod(mut n: u32, mut d: u32) -> uidiv_t {
	let mut ret = 0;
	let mut add = 1;
	while n / 2 >= d && add != 0 { d <<= 1; add <<= 1; }
	while add > 0 { if n >= d { ret += add; n -= d; } add  >>= 1; d >>= 1; }

	uidiv_t { quo: ret, rem: n, }
}

#[no_mangle]
pub extern fn __aeabi_uidiv(n: u32, d: u32) -> u32 {
	__aeabi_uidivmod(n, d).quo
}
#[no_mangle]
pub extern fn __umodsi3(n: u32, d: u32) -> u32 {
	__aeabi_uidivmod(n, d).rem
}

