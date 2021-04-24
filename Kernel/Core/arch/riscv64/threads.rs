//! RISCV thread handling

pub struct State {
	pt_root: u64,
	sp: usize,
	#[allow(dead_code)]
	stack_handle: Option< ::memory::virt::ArrayHandle<u8> >,
}
impl State
{
	pub fn new(a: &super::memory::virt::AddressSpace) -> State {
		State {
			pt_root: a.as_phys(),
			sp: 0,
			stack_handle: None,	// Initialised on thread start
		}
	}
}
pub fn init_tid0_state() -> State {
	State {
		pt_root: super::memory::virt::AddressSpace::pid0().as_phys(),
		sp: 0,
		stack_handle: None,
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
	stack.push(thread_trampoline as usize);	// An assembly trampoline that pops the above two words and calls `thread_root`
	stack.push(0);	// GP
	stack.push(0);	// TP
	// - s0-s11
	stack.push([0usize; 12]);
	// - fs0-fs11
	//for _ in 0 ..= 11 {
	//	stack.push(0usize);
	//}
	
	// Apply newly updated state
	let (stack_handle, stack_pos) = stack.unwrap();
	thread.cpu_state.sp = stack_pos;
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
		alloc: ::memory::virt::ArrayHandle<u8>,
		top: usize,
	}
	impl StackInit {
		fn new() -> StackInit {
			let ah = ::memory::virt::alloc_stack().into_array::<u8>();
			StackInit {
				top: &ah[ah.len()-1] as *const _ as usize + 1,
				alloc: ah,
			}
		}
		fn unwrap(self) -> (::memory::virt::ArrayHandle<u8>, usize) {
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



pub fn idle() {
	// SAFE: Just waits for an interrupt
	unsafe { asm!("wfi") }
}
pub fn switch_to(thread: ::threads::ThreadPtr) {
	#[allow(improper_ctypes)]
	extern "C" {
		fn task_switch(old_sp: &mut usize, new_sp: usize, satp: u64, thread_ptr: usize);
	}
	// SAFE: Pointer access is valid, task_switch should be too
	unsafe
	{
		let outstate = &mut (*(borrow_thread() as *mut crate::threads::Thread)).cpu_state;
		let new_sp = thread.cpu_state.sp;
		let new_satp = (thread.cpu_state.pt_root >> 12) | (8 << 60);
		log_trace!("Switching to SP={:#x},SATP={:#x}", new_sp, new_satp);
		task_switch(&mut outstate.sp, new_sp, new_satp, thread.into_usize());
	}
}

pub fn get_idle_thread() -> crate::threads::ThreadPtr {
	todo!("get_idle_thread");
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
pub fn borrow_thread() -> *const ::threads::Thread {
	let rv: *const ::threads::Thread;
	// SAFE: Atomic read from a per-CPU scratch register
	unsafe { asm!("csrr {}, sscratch", out(reg) rv, options(nomem, pure)); }
	//use super::{puts,puth}; puts("borrow_thread: 0x"); puth(rv as usize as u64); puts("\n");
	rv
}
