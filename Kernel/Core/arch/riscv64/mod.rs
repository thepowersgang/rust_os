// "Tifflin" Kernel
// - By John Hodge (Mutabah/thePowersGang)
//
// Core/arch/riscv64/main.rs
//! RISC-V architecture bindings

pub mod memory;

pub mod sync {
	pub struct SpinlockInner(());
	impl SpinlockInner
	{
		pub const fn new() -> SpinlockInner {
			SpinlockInner( () )
		}
		pub fn inner_lock(&self) {
		}
		pub fn try_inner_lock_cpu(&self) -> bool {
			false
		}
		pub fn inner_release(&self) {
		}
	}


	pub struct HeldInterrupts;
	pub fn hold_interrupts() -> HeldInterrupts {
		HeldInterrupts
	}

	pub unsafe fn start_interrupts() {
	}
	pub unsafe fn stop_interrupts() {
	}
}
pub mod interrupts {
	#[derive(Default)]
	pub struct IRQHandle;
	#[derive(Debug)]
	pub struct BindError;

	pub fn bind_gsi(gsi: usize, handler: fn(*const ()), info: *const ()) -> Result<IRQHandle, BindError>
	{
		Err(BindError)
	}
}

pub mod boot;

pub mod pci {
	pub fn read(addr: u32) -> u32 {
		!0
	}
	pub fn write(addr: u32, val: u32) {
		todo!("pci::write");
	}
}

pub mod threads {
	pub struct State;
	impl State
	{
		pub fn new(a: &super::memory::virt::AddressSpace) -> State {
			todo!("");
		}
	}
	pub fn init_tid0_state() -> State {
		State
	}
	pub fn start_thread<F: FnOnce()+Send+'static>(thread: &crate::threads::Thread, code: F)
	{
	}

	pub fn idle() {
		// SAFE: Just waits for an interrupt
		unsafe { asm!("wfi") }
	}
	pub fn switch_to(t: ::threads::ThreadPtr) {
	}

	pub fn get_idle_thread() -> crate::threads::ThreadPtr {
		todo!("");
	}

	pub fn set_thread_ptr(t: ::threads::ThreadPtr) {
		// SAFE: Atomic write to a per-CPU scratch register
		unsafe {
			asm!("csrw sscratch, {}", in(reg) t.into_usize());
		}
	}
	pub fn get_thread_ptr() -> Option<::threads::ThreadPtr> {
		let ret: usize;
		// SAFE: Atomic read from a per-CPU scratch register
		unsafe { asm!("csrr {}, sscratch", out(reg) ret, options(nomem, pure)); }
		if ret == 0 {
			None
		}
		else {
			// SAFE: Stored value assumed to be valid
			unsafe {
				Some(crate::threads::ThreadPtr::from_usize(ret))
			}
		}
	}
	pub fn borrow_thread() -> *const ::threads::Thread {
		let rv: *const ::threads::Thread;
		// SAFE: Atomic read from a per-CPU scratch register
		unsafe { asm!("csrr {}, sscratch", out(reg) rv, options(nomem, pure)); }
		rv
	}
}

pub mod x86_io {
	pub unsafe fn inb(_p: u16) -> u8 { panic!("calling inb on non-x86") }
	pub unsafe fn inw(_p: u16) -> u16 { panic!("calling inw on non-x86") }
	pub unsafe fn inl(_p: u16) -> u32 { panic!("calling inl on non-x86") }
	pub unsafe fn outb(_p: u16, _v: u8) {}
	pub unsafe fn outw(_p: u16, _v: u16) {}
	pub unsafe fn outl(_p: u16, _v: u32) {}
}

pub fn puts(s: &str) {
	for b in s.bytes() {
		putb(b);
	}
}
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
fn putb(v: u8) {
	const UART_PTR: *mut u8 = 0xFFFFFFFF_40000000 as *mut u8;
	// SAFE: Just writes to the FIFO
	unsafe {
		// Wait for free space in the FIFO (TODO: What bit to check?)
		// IDEA - Keep an atomic counter, increment to 16 and once reached spin until FIFO empty bit
		// > Check FIFO empty, if empty clear
		//while ::core::ptr::volatile_read(UART_PTR.offset(5)) & (1 << 6) != 0 {
		//}
		::core::ptr::write_volatile(UART_PTR.offset(0), v);
	}
}

pub fn print_backtrace() {
}

pub fn cur_timestamp() -> u64 {
	0
}

pub fn drop_to_user(entry: usize, stack: usize, args_len: usize) -> ! {
	loop {}
}
