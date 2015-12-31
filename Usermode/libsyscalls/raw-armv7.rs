
	macro_rules! syscall_a {
		($id:expr, $( $reg:tt = $val:expr),*) => {{
			let rv_l: usize;
			let rv_h: usize;
			asm!("swi #0"
				: "={r0}" (rv_l), "={r1}" (rv_h)
				: "{r12}" ($id as usize) $(, $reg ($val as usize))*
				: "r0","r1","r2","r3"
				: "volatile"
				);
			(rv_h as u64) << 32 | (rv_l as u64)
		}};
	}
	// SAVE r1, r2, r3, r4, r5, r6
	#[inline]
	pub unsafe fn syscall_0(id: u32) -> u64 {
		syscall_a!(id, )
	}
	#[inline]
	pub unsafe fn syscall_1(id: u32, a1: usize) -> u64 {
		syscall_a!(id, "{r0}"=a1)
	}
	#[inline]
	pub unsafe fn syscall_2(id: u32, a1: usize, a2: usize) -> u64 {
		syscall_a!(id, "{r0}"=a1, "{r1}"=a2)
	}
	#[inline]
	pub unsafe fn syscall_3(id: u32, a1: usize, a2: usize, a3: usize) -> u64 {
		syscall_a!(id, "{r0}"=a1, "{r1}"=a2, "{r2}"=a3)
	}
	#[inline]
	pub unsafe fn syscall_4(id: u32, a1: usize, a2: usize, a3: usize, a4: usize) -> u64 {
		syscall_a!(id, "{r0}"=a1, "{r1}"=a2, "{r2}"=a3, "{r3}"=a4)
	}
	#[inline]
	pub unsafe fn syscall_5(id: u32, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize) -> u64 {
		syscall_a!(id, "{r0}"=a1, "{r1}"=a2, "{r2}"=a3, "{r3}"=a4, "{r4}"=a5)
	}
	#[inline]
	pub unsafe fn syscall_6(id: u32, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize) -> u64 {
		syscall_a!(id, "{r0}"=a1, "{r1}"=a2, "{r2}"=a3, "{r3}"=a4, "{r4}"=a5, "{r5}"=a6)
	}
