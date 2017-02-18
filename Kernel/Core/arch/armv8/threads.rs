// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/arch/armv8/threads.rs
// - ARMv8 (AArch64) interface

pub struct State
{
	sp: usize,
	ttbr0: u64,
	stack_handle: Option< ::memory::virt::ArrayHandle<u8> >,
}

pub fn init_tid0_state() -> State
{
	State {
		sp: 0,
		ttbr0: super::memory::virt::AddressSpace::pid0().as_phys(),
		stack_handle: None,
		}
}

impl State
{
	pub fn new(addr_space: &::memory::virt::AddressSpace) -> State {
		State {
			sp: 0,
			ttbr0: addr_space.as_phys(),
			stack_handle: None,
			}
	}
}

pub fn get_idle_thread() -> ::threads::ThreadPtr {
	todo!("get_idle_thread");
}

pub fn set_thread_ptr(thread: ::threads::ThreadPtr) {
	// SAFE: Write to per-CPU register
	unsafe {
		asm!("msr TPIDR_EL1, $0" : : "r"(thread.into_usize()));
	}
}
pub fn get_thread_ptr() -> Option<::threads::ThreadPtr> {
	let ret: usize;
	// SAFE: Read-only access to a per-cpu register
	unsafe {
		asm!("mrs $0, TPIDR_EL1" : "=r"(ret));
	}
	if ret == 0 {
		None
	}
	else {
		// SAFE: Stored value assumed to be valid
		unsafe {
			Some(::threads::ThreadPtr::from_usize(ret))
		}
	}
}
fn borrow_thread_mut() -> *mut ::threads::Thread {
	let ret;
	// SAFE: Read-only access to a per-cpu register
	unsafe {
		asm!("mrs $0, TPIDR_EL1" : "=r"(ret));
	}
	ret
}
pub fn borrow_thread() -> *const ::threads::Thread {
	borrow_thread_mut() as *const _
}
pub fn switch_to(thread: ::threads::ThreadPtr) {
	#[allow(improper_ctypes)]
	extern "C" {
		fn task_switch(old_sp: &mut usize, new_sp: usize, ttbr0: u64, thread_ptr: usize);
	}
	// SAFE: Pointer access is valid, task_switch should be too
	unsafe
	{
		let outstate = &mut (*borrow_thread_mut()).cpu_state;
		let new_sp = thread.cpu_state.sp;
		let new_ttbr0 = thread.cpu_state.ttbr0;
		log_trace!("Switching to SP={:#x},TTBR0={:#x}", new_sp, new_ttbr0);
		task_switch(&mut outstate.sp, new_sp, new_ttbr0, thread.into_usize());
	}
}
pub fn idle() {
	log_trace!("idle");
	// SAFE: Calls 'wait for interrupt'
	unsafe {
		asm!("wfi" : : : : "volatile");
	}
}

pub fn start_thread<F: FnOnce()+Send+'static>(thread: &mut ::threads::Thread, code: F) {
	let mut stack = StackInit::new();

	// 2. Populate stack with `code`
	stack.push(code);
	let a = stack.pos();
	stack.align(16);
	// State for `thread_trampoline`
	stack.push(a);
	stack.push( thread_root::<F> as usize );
	
	// 3. Populate with task_switch state
	// - R19-R28 saved by task_switch
	for _ in 19 .. 28+1 {
		stack.push(0_usize);
	}
	// - LR popped by task_switch - Trampoline that sets R0 to the address of 'code'
	stack.push( thread_trampoline as usize );	// R30 - aka LR
	stack.push(0_usize);	// R29

	stack.push(0_usize);	// TPIDR_EL0 - User Thread Pointer
	stack.push(0_usize);	// SP_EL0    - User SP
	stack.push(0_usize);	// pad
	stack.push(0_usize);	// ELR_EL1 - Exception return
	
	// 4. Apply newly updated state
	let (stack_handle, stack_pos) = stack.unwrap();
	thread.cpu_state.sp = stack_pos;
	thread.cpu_state.stack_handle = Some(stack_handle);

	// END: Parent function will run this thread for us
	
	extern "C" {
		fn thread_trampoline();
	}
	fn thread_root<F: FnOnce()+Send>(code_ptr: *const F) -> ! {
		// 1. Run closure
		// SAFE: Functionally owns that pointer
		(unsafe { ::core::ptr::read(code_ptr) })();
		// 2. terminate thread
		panic!("TODO: Terminate thread at end of thread_root");
	}
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


