
	macro_rules! syscall_a {
		($id:expr, $( $reg:tt = $val:expr),*) => {{
			let rv;
			asm!("syscall"
				: "={rax}" (rv)
				: "{rax}" ($id) $(, $reg ($val))*
				: "rcx", "r11"
				: "volatile"
				);
			rv
		}};
	}
	pub unsafe fn syscall_0(id: u32) -> u64 {
		syscall_a!(id, )
	}
	pub unsafe fn syscall_1(id: u32, a1: usize) -> u64 {
		syscall_a!(id, "{rdi}"=a1)
	}
	pub unsafe fn syscall_2(id: u32, a1: usize, a2: usize) -> u64 {
		syscall_a!(id, "{rdi}"=a1, "{rsi}"=a2)
	}
