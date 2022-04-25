// "Tifflin" Kernel
// - By John Hodge (Mutabah/thePowersGang)
//
// Core/arch/riscv64/main.rs
//! RISC-V architecture bindings
use ::core::sync::atomic::{AtomicUsize};

module_define!{ arch, [], init }
fn init()
{
	// Start the FDT bus enumeration, informing it of the interrupt controller
	fdt_devices::init(interrupts::get_intc);
}

#[path="../armv7/fdt_devices.rs"]
mod fdt_devices;

mod sbi;

//mod backtrace_dwarf;

pub mod memory;

pub mod sync {
	use ::core::sync::atomic::{AtomicBool, Ordering};
	pub struct SpinlockInner
	{
		flag: AtomicBool,
	}
	impl SpinlockInner
	{
		pub const fn new() -> SpinlockInner {
			SpinlockInner { flag: AtomicBool::new(false) }
		}

		pub fn inner_lock(&self)
		{
			while self.flag.swap(true, Ordering::Acquire) {
				// TODO: Once SMP is a thing, this should spin.
				super::puts("Contented lock!"); super::puth(self as *const _ as usize as u64);
				panic!("Contended {:p}", self);
				//loop {}
			}
		}
		pub unsafe fn inner_release(&self)
		{
			assert!( self.flag.swap(false, Ordering::Release) == true, "Releasing an unlocked spinlock?" );
			assert!( self.flag.load(Ordering::Relaxed) == false );
		}

		pub fn try_inner_lock_cpu(&self) -> bool
		{
			// TODO: Ensure that this CPU isn't holding the lock
			if self.flag.swap(true, Ordering::Acquire) == false {
				true
			}
			else {
				false
			}
		}
	}


	pub struct HeldInterrupts(u64);
	pub fn hold_interrupts() -> HeldInterrupts {
		// SAFE: Only reads from a CSR
		unsafe {
			let v: u64;
			::core::arch::asm!("csrrci {}, sstatus, 0x2", lateout(reg) v);	// Clear SIE and return the original contents
			HeldInterrupts(v & 0x2)
		}
	}
	impl ::core::ops::Drop for HeldInterrupts {
		fn drop(&mut self) {
			// SAFE: Only sets SIE (restoring it to previous state)
			unsafe {
				assert!(self.0 & !0x2 == 0);	// only SIE (bit 2) should be set
				::core::arch::asm!("csrs sstatus, {}", in(reg) self.0);
			}
		}
	}

	pub unsafe fn start_interrupts() {
		::core::arch::asm!("csrsi sstatus, 0x2");
	}
	pub unsafe fn stop_interrupts() {
		::core::arch::asm!("csrci sstatus, 0x2");
	}
}


mod plic;

pub mod interrupts
{
	use ::core::sync::atomic::{Ordering, AtomicUsize};
	use super::plic::PlicInstance;
	use super::fdt_devices;

	#[derive(Default)]
	pub struct IRQHandle(usize);
	#[derive(Debug)]
	pub struct BindError;

	pub(super) fn get_intc(compat: fdt_devices::Compat, reg: fdt_devices::Reg)->Option<&'static dyn fdt_devices::IntController>
	{
		if compat.matches("riscv,plic0")
		{
			if PLIC.is_init() {
				log_error!("Two PLIC instances?");
				return None;
			}
			let ah = reg.iter_paddr()
				// SAFE: Trusting the FDT
				.map(|r| unsafe { let (base,size) = r.expect("PLIC MMIO out of PAddr range"); crate::memory::virt::map_mmio(base, size).expect("PLIC MMIO map failed") })
				.next().expect("PLIC had no MMIO ranges in FDT?")
				;
			PLIC.init(ah);

			// Enable all interrupts
			for (i,slot) in INTERRUPT_HANDLES.iter().enumerate()
			{
				let cb = slot.0.load(Ordering::SeqCst);
				// If >1 it's already been initialised (if 1, then it's currently being initialised - so will be enabled by `bind_gsi`
				if cb > 1
				{
					PLIC.set_enable(i, true);
				}
			}
			// Handle any pending ones
			self::handle();

			// Return a stub instance
			struct Instance;
			impl fdt_devices::IntController for Instance {
				fn get_gsi(&self, mut cells: fdt_devices::Cells) -> Option<u32> {
					cells.read_1()
				}
			}
			Some(&Instance)
		}
		else
		{
			None
		}
	}

	macro_rules! array_1024 {
		($e:expr) => { array_1024!(@1 $e, $e) };
		(@1 $($e:tt)*) => { array_1024!(@2 $($e)*, $($e)*) };
		(@2 $($e:tt)*) => { array_1024!(@3 $($e)*, $($e)*) };
		(@3 $($e:tt)*) => { array_1024!(@4 $($e)*, $($e)*) };
		(@4 $($e:tt)*) => { array_1024!(@5 $($e)*, $($e)*) };
		(@5 $($e:tt)*) => { array_1024!(@6 $($e)*, $($e)*) };
		(@6 $($e:tt)*) => { array_1024!(@7 $($e)*, $($e)*) };
		(@7 $($e:tt)*) => { array_1024!(@8 $($e)*, $($e)*) };
		(@8 $($e:tt)*) => { array_1024!(@e $($e)*, $($e)*) };
		(@e $($e:tt)*) => { [ $($e)*, $($e)* ] };
	}
	static INTERRUPT_HANDLES: [ (AtomicUsize, AtomicUsize); 1024 ] = array_1024!( (AtomicUsize::new(0), AtomicUsize::new(0)) );
	static PLIC: PlicInstance = PlicInstance::new_uninit();

	pub fn bind_gsi(gsi: usize, handler: fn(*const ()), info: *const ()) -> Result<IRQHandle, BindError>
	{
		let slot = &INTERRUPT_HANDLES[gsi];
		match slot.0.compare_exchange(0, 1, Ordering::SeqCst, Ordering::Relaxed)
		{
		Ok(_) => {
			log_debug!("bind_gsi({}) = {:p} {:p}", gsi, handler, info);
			slot.1.store(info as usize, Ordering::Relaxed);
			slot.0.store(handler as usize, Ordering::Relaxed);
			if PLIC.is_init() {
				PLIC.set_enable(gsi, true);
			}
			Ok( IRQHandle(gsi+1) )
			},
		Err(existing) => {
			log_warning!("bind_gsi({}) = CONFLICT {:#x}", gsi, existing);
			Err(BindError)
			},
		}
	}

	impl ::core::ops::Drop for IRQHandle {
		fn drop(&mut self)
		{
			if self.0 > 0
			{
				let gsi = self.0 - 1;
				let slot = &INTERRUPT_HANDLES[gsi];
				assert!( slot.0.swap(0, Ordering::SeqCst) > 1, "Unbinding IRQ handle that is already empty - gsi={}", gsi);
				if PLIC.is_init() {
					PLIC.set_enable(gsi, false);
				}
			}
		}
	}

	pub(super) fn handle()
	{
		assert!(PLIC.is_init());
		PLIC.claim_complete_cycle(|idx| {
			let slot = &INTERRUPT_HANDLES[idx];
			log_debug!("IRQ{}", idx);
			let info = slot.1.load(Ordering::SeqCst);
			let cb = slot.0.load(Ordering::SeqCst);
			if cb > 1 {
				// SAFE: Correct type, pointer set by `bind_gsi` above
				let cb: fn(*const ()) = unsafe { ::core::mem::transmute(cb) };
				cb(info as *const ());
			}
			else if cb == 1 {
			}
			else {
			}
			});
	}

	pub(super) fn wait_for_interrupt()
	{
		if PLIC.is_init()
		{
			// SAFE: Just waits for an interrupt
			unsafe { ::core::arch::asm!("wfi; csrsi sstatus, 0x2") }
		}
		else
		{
			// While the PLIC is initialising, busy trigger all registered IRQ handlers to force polling
			for slot in INTERRUPT_HANDLES.iter()
			{
				let info = slot.1.load(Ordering::SeqCst);
				let cb = slot.0.load(Ordering::SeqCst);
				if cb > 1 {
					// SAFE: Correct type, pointer set by `bind_gsi` above
					let cb: fn(*const ()) = unsafe { ::core::mem::transmute(cb) };
					cb(info as *const ());
				}
				else if cb == 1 {
				}
				else {
				}
			}
			// TODO: Set timer interrupt and wait for that? (to reduce CPU load)
			// - Might work without the PLIC?
			// This codepath shouldn't really be run anymore (was needed before FDT was reworked) to use get_intc
		}
	}
}

pub mod boot;

pub mod threads;

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

pub mod time {
	pub fn cur_timestamp() -> u64 {
		let v: u64;
		// SAFE: Reading a CSR with no side-effects
		unsafe { ::core::arch::asm!("rdtime {}", lateout(reg) v); }
		v / 10000//_000	// FDT: "" "cpus" ".timebase-frequency"
	}

	pub fn request_tick(time: u64) {
	}
}

pub fn drop_to_user(entry: usize, stack: usize, args_len: usize) -> ! {
	// Create an exception frame
	// SAFE: Validated 
	unsafe {
		::core::arch::asm!("
			csrc sstatus,{3}
			csrs sstatus,{4}
			csrw sepc, {0}
			mv sp, {1}
			mv a0, {2}
			sret
			",
			in(reg) entry, in(reg) stack, in(reg) args_len,
			in(reg) /*mask*/0x142,	// SPP, SPIE, SIE
			in(reg) /*new */0x040,	// SPIE (keep interrupts disabled until sret completes)
			options(noreturn)
			);
	}
}

#[repr(C)]
struct HartState
{
	/// Scratch space used to store `t1` during trap handler
	scratch_t1: u64,	// Actually mutated by the assembly stub
	/// Kernel's base SP value (loaded when entering from usermode)
	kernel_base_sp: AtomicUsize,	// Read by assembly stub
	/// Currently executing thread
	current_thread: AtomicUsize,
	/// This CPU's idle thread
	idle_thread: AtomicUsize,
}
#[no_mangle]
static HART0_STATE: HartState = HartState {
	scratch_t1: 0,
	kernel_base_sp: AtomicUsize::new(memory::addresses::STACK0_BASE),
	current_thread: AtomicUsize::new(0),
	idle_thread: AtomicUsize::new(0),
	};
impl HartState
{
	fn get_current() -> &'static HartState {
		// SAFE: Reads a valid CSR, the pointer contained within should be valid
		unsafe {
			let ptr: *const HartState;
			::core::arch::asm!("csrr {}, sscratch", out(reg) ptr);
			&*ptr
		}
	}
}

#[repr(C)]
struct FaultRegs
{
	sstatus: u64,
	stval: u64,
	sepc: u64,
	scause: u64,
	regs: [u64; 31],
}
static REG_NAMES: [&'static str; 31] = [
	"RA",
	"SP",
	"GP",
	"TP",
	"T0",
	"T1",
	"T2",
	"S0",
	"S1",
	"A0",
	"A1",
	"A2",
	"A3",
	"A4",
	"A5",
	"A6",
	"A7",
	"S2",
	"S3",
	"S4",
	"S5",
	"S6",
	"S7",
	"S8",
	"S9",
	"S10",
	"S11",
	"T3",
	"T4",
	"T5",
	"T6",
	];
#[no_mangle]
extern "C" fn trap_vector_rs(state: &mut FaultRegs)
{
	if state.scause >> 63 != 0 {
		// IRQ
		match state.scause & !0 >> 1
		{
		// Software
		0..=3 => {},
		// Timer
		4..=7 => {},
		// External
		8..=11 => return interrupts::handle(),
		_ => {},
		}
	}
	// Environemnt call from U-mode
	if state.scause == 8 {
		extern "C" {
			fn syscalls_handler(id: u32, first_arg: *const usize, count: u32) -> u64;
		}
		// SAFE: Correct inputs
		unsafe {
			state.regs[10-1] = syscalls_handler(state.regs[10-1] as u32, &state.regs[11-1] as *const u64 as *const usize, 7-1);
		}
		state.sepc += 4;	// ECALL doesn't have a compressed format
		return ;
	}

	// Page fault (write)
	if state.scause == 15
	{
		// Check for a CoW page
		if memory::virt::page_fault(state.stval as usize, /*is_write*/state.scause == 15)
		{
			// If successful, repeat the instruction
			return ;
		}
	}

	let reason = match state.scause
		{
		0 => "Instruction address misaligned",
		1 => "Instruction access fault",
		2 => "Illegal instruction",
		3 => "Breakpoint",
		4 => "Load address misaligned",
		5 => "Load access fault",
		6 => "Store/AMO address misaligned",
		7 => "Store/AMO access fault",
		8 => "Environment call from U-mode",
		9 => "Environment call from S-mode",
		10 => "/Reserved for future standard use/",
		11 => "/Reserved for future standard use/",
		12 => "Instruction page fault",
		13 => "Load page fault",
		15 => "Store/AMO page fault",
		16..=23 => "/Reserved for future standard use/",
		24..=31 => "/Reserved for future custom use/",
		32..=47 => "/Reserved for future standard use/",
		48..=63 => "/Reserved for future custom use/",
		_ => "/Reserved for future standard use/",
		};
	log_error!("FAULT: {:#x} {} at {:#x} stval={:#x}", state.scause, reason, state.sepc, state.stval);
	let mut it = Iterator::chain( [(&"r0",&0)].iter().copied(), Iterator::zip( REG_NAMES.iter(), state.regs.iter() ));
	for _ in 0..32/4 {
		let (r1,v1) = it.next().unwrap();
		let (r2,v2) = it.next().unwrap();
		let (r3,v3) = it.next().unwrap();
		let (r4,v4) = it.next().unwrap();
		log_error!("{:3}={:16x} {:3}={:16x} {:3}={:16x} {:3}={:16x}", r1,v1, r2,v2, r3,v3, r4,v4);
	}
	loop {
		// SAFE: No side-effects to WFI
		unsafe { ::core::arch::asm!("wfi"); }
	}
}
