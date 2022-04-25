//! RISCV thread handling
use ::core::sync::atomic::Ordering;

pub struct State {
	pt_root: u64,
	kernel_base_sp: usize,
	sp: usize,
	#[allow(dead_code)]
	stack_handle: Option< crate::memory::virt::ArrayHandle<u8> >,
}
impl State
{
	pub fn new(a: &crate::arch::memory::virt::AddressSpace) -> State {
		State {
			pt_root: a.inner().as_phys(),
			sp: 0,
			kernel_base_sp: 0,
			stack_handle: None,	// Initialised on thread start
		}
	}
}
pub fn init_tid0_state() -> State {
	State {
		pt_root: super::memory::virt::AddressSpace::pid0().as_phys(),
		sp: 0,
		kernel_base_sp: super::memory::addresses::STACK0_BASE,
		stack_handle: None,
	}
}
pub fn init_smp() {
	// TODO: Check for other cores in the FDT and use SBI to start them
	super::sbi::dump_sbi_info();

	//crate::device_manager::register_driver();
	if let Some(fdt) = super::boot::get_fdt()
	{
		for cpu in fdt.get_props_cb(|ofs,_leaf,name| {
			match ofs
			{
			0 => name == "cpus",
			1 => name == "cpu" || name.starts_with("cpu@"),
			2 => name == "reg",
			_ => false,
			}
			})
		{
			log_debug!("cpu = {:?}", cpu);
			//super::sbi::hart_management::start();
		}
	}
}
pub fn start_thread<F: FnOnce()+Send+'static>(thread: &mut crate::threads::Thread, code: F)
{
	// Prepare a stack that matches the layout expected by `switch_to`
	let mut stack = StackInit::new();
	stack.push(code);
	let a = stack.pos();
	stack.align(8);
	stack.push(a);	// Data pointer
	stack.push(thread_root::<F> as usize);
	// - ra, gp, tp
	stack.push(thread_trampoline as usize);	// RA = An assembly trampoline that pops the above two words and calls `thread_root`
	stack.push(0usize);	// GP
	stack.push(0usize);	// TP
	// - s0-s11
	stack.push([0usize; 12]);
	// - fs0-fs11
	//for _ in 0 ..= 11 {
	//	stack.push(0usize);
	//}
	
	// Apply newly updated state
	let (stack_handle, stack_pos) = stack.unwrap();
	thread.cpu_state.sp = stack_pos;
	thread.cpu_state.kernel_base_sp = stack_pos;	// TODO: End of the stack alloc instead?
	thread.cpu_state.stack_handle = Some(stack_handle);

	return ;

	extern "C" {
		fn thread_trampoline() -> !;
	}
	fn thread_root<F: FnOnce()+Send+'static>(code_ptr: *mut F)->! {
		// 1. Run closure
		// SAFE: Functionally owns that pointer
		(unsafe { ::core::ptr::read(code_ptr) })();
		// 2. terminate thread
		panic!("TODO: Terminate thread at end of thread_root");
	}

	struct StackInit {
		alloc: crate::memory::virt::ArrayHandle<u8>,
		top: usize,
	}
	impl StackInit {
		fn new() -> StackInit {
			let ah = crate::memory::virt::alloc_stack().into_array::<u8>();
			StackInit {
				top: &ah[ah.len()-1] as *const _ as usize + 1,
				alloc: ah,
			}
		}
		fn unwrap(self) -> (crate::memory::virt::ArrayHandle<u8>, usize) {
			(self.alloc, self.top)
		}
		fn push<T: 'static>(&mut self, v: T) {
			assert!(self.top > &self.alloc[0] as *const _ as usize);
			let mut p = self.top;
			p -= ::core::mem::size_of::<T>();
			p -= p % ::core::mem::align_of::<T>();
			assert!(p >= &self.alloc[0] as *const _ as usize);
			// SAFE: Pointer is valid and data is of correct lifetime
			unsafe {
				::core::ptr::write(p as *mut T, v);
				//log_trace!("push() {:p} = {:x?}", p as *mut T, ::core::slice::from_raw_parts(p as *const u8, ::core::mem::size_of::<T>()));
			}
			self.top = p;
		}
		fn pos(&self) -> usize {
			self.top
		}
		fn align(&mut self, bytes: usize) {
			let mut p = self.top;
			p -= p % bytes;
			self.top = p;
		}
	}
}



pub fn idle(held_interrupts: crate::arch::sync::HeldInterrupts) {
	// wait_for_interrupt ensures that interrupts are enabled
	::core::mem::forget(held_interrupts);
	// Idling done in th IRQ module, so it can handle the driver not yet being up
	super::interrupts::wait_for_interrupt();
}
pub fn switch_to(thread: crate::threads::ThreadPtr) {
	#[allow(improper_ctypes)]
	extern "C" {
		fn task_switch(old_sp: &mut usize, new_sp: usize, satp: u64);
	}
	// SAFE: Pointer access is valid, task_switch should be too
	unsafe
	{
		let outstate = &mut (*(borrow_thread() as *mut crate::threads::Thread)).cpu_state;
		let new_sp = thread.cpu_state.sp;
		let new_satp = (thread.cpu_state.pt_root >> 12) | (8 << 60);
		{
			let hart_state = super::HartState::get_current();
			hart_state.kernel_base_sp.store(thread.cpu_state.kernel_base_sp, Ordering::SeqCst);
			hart_state.current_thread.store(thread.into_usize(), Ordering::SeqCst);
		}
		log_trace!("Switching to SP={:#x},SATP={:#x}", new_sp, new_satp);
		task_switch(&mut outstate.sp, new_sp, new_satp);
	}
}

// Get the idle thread for this HART
pub fn get_idle_thread() -> crate::threads::ThreadPtr {
	// SAFE: Valid transmutes
	unsafe
	{
		let state = super::HartState::get_current();
		let mut ptr = state.idle_thread.load(Ordering::Relaxed);
		if ptr == 0
		{
			ptr = ::core::mem::transmute( crate::threads::new_idle_thread(0) );
			state.idle_thread.store(ptr, Ordering::Relaxed);
		}
		::core::mem::transmute(ptr)
	}
}

pub fn set_thread_ptr(t: crate::threads::ThreadPtr) {
	super::HartState::get_current().current_thread.store(t.into_usize(), Ordering::SeqCst);
}
pub fn get_thread_ptr() -> Option<crate::threads::ThreadPtr> {
	let ret = super::HartState::get_current().current_thread.load(Ordering::SeqCst) as usize;
	//use super::{puts,puth}; puts("get_thread_ptr: 0x"); puth(ret as u64); puts("\n");
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
pub fn borrow_thread() -> *const crate::threads::Thread {
	let ret = super::HartState::get_current().current_thread.load(Ordering::SeqCst) as usize;
	ret as *const crate::threads::Thread
}
