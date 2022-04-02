
pub const PAGE_SIZE: usize = 0x4000;

	macro_rules! syscall_a {
		($id:expr, $( $reg:tt = $val:expr),*) => {{
			let rv: usize;
			::core::arch::asm!("svc #0",
				lateout("x0") rv,
				in("x12") ($id as usize) $(, in($reg) ($val as usize))*,
				lateout("x1") _, lateout("x2") _, lateout("x3") _, lateout("x4") _, lateout("x5") _, lateout("x6") _, lateout("x7") _
				);
			rv as u64
		}};
	}
	// SAVE x1, x2, x3, x4, x5, r6
	#[inline(always)]
	pub unsafe fn syscall_0(id: u32) -> u64 {
		syscall_a!(id, )
	}
	#[inline(always)]
	pub unsafe fn syscall_1(id: u32, a1: usize) -> u64 {
		syscall_a!(id, "x0"=a1)
	}
	#[inline(always)]
	pub unsafe fn syscall_2(id: u32, a1: usize, a2: usize) -> u64 {
		syscall_a!(id, "x0"=a1, "x1"=a2)
	}
	#[inline(always)]
	pub unsafe fn syscall_3(id: u32, a1: usize, a2: usize, a3: usize) -> u64 {
		syscall_a!(id, "x0"=a1, "x1"=a2, "x2"=a3)
	}
	#[inline(always)]
	pub unsafe fn syscall_4(id: u32, a1: usize, a2: usize, a3: usize, a4: usize) -> u64 {
		syscall_a!(id, "x0"=a1, "x1"=a2, "x2"=a3, "x3"=a4)
	}
	#[inline(always)]
	pub unsafe fn syscall_5(id: u32, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize) -> u64 {
		syscall_a!(id, "x0"=a1, "x1"=a2, "x2"=a3, "x3"=a4, "x4"=a5)
	}
	#[inline(always)]
	pub unsafe fn syscall_6(id: u32, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize) -> u64 {
		syscall_a!(id, "x0"=a1, "x1"=a2, "x2"=a3, "x3"=a4, "x4"=a5, "x5"=a6)
	}
