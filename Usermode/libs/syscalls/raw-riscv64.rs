pub const PAGE_SIZE: usize = 0x1000;

macro_rules! syscall_a {
	($id:expr, $( $reg:tt = $val:expr),*) => {{
		let rv;
		::core::arch::asm!("ecall",
			lateout("a0") rv,
			in("a0") ($id as usize) $(, in($reg) ($val as usize))*,
			options(nostack)
			);
		rv
	}};

}
#[inline]
pub unsafe fn syscall_0(id: u32) -> u64 {
	syscall_a!(id, )
}
#[inline]
pub unsafe fn syscall_1(id: u32, a1: usize) -> u64 {
	syscall_a!(id, "a1"=a1)
}
#[inline]
pub unsafe fn syscall_2(id: u32, a1: usize, a2: usize) -> u64 {
	syscall_a!(id, "a1"=a1, "a2"=a2)
}
#[inline]
pub unsafe fn syscall_3(id: u32, a1: usize, a2: usize, a3: usize) -> u64 {
	syscall_a!(id, "a1"=a1, "a2"=a2, "a3"=a3)
}
#[inline]
pub unsafe fn syscall_4(id: u32, a1: usize, a2: usize, a3: usize, a4: usize) -> u64 {
	syscall_a!(id, "a1"=a1, "a2"=a2, "a3"=a3, "a4"=a4)
}
#[inline]
pub unsafe fn syscall_5(id: u32, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize) -> u64 {
	syscall_a!(id, "a1"=a1, "a2"=a2, "a3"=a3, "a4"=a4, "a5"=a5)
}
#[inline]
pub unsafe fn syscall_6(id: u32, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize) -> u64 {
	syscall_a!(id, "a1"=a1, "a2"=a2, "a3"=a3, "a4"=a4, "a5"=a5, "a6"=a6)
}

