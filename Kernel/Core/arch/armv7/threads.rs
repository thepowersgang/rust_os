//! ARMv7 thread handling

#[derive(Default)]
pub struct State {
	sp: usize,
	ttbr0: u32,
	stack_handle: Option< ::memory::virt::ArrayHandle<u8> >,
}

impl State
{
	pub fn new(address_space: &::memory::virt::AddressSpace) -> State {
		State {
			sp: 0,
			ttbr0: address_space.get_ttbr0(),
			stack_handle: None,
		}
	}
}

pub fn init_tid0_state() -> State {
	extern "C" {
		static kernel_table0: ::Void;
	}
	State {
		ttbr0: ::memory::virt::get_phys( &kernel_table0 ),
		..State::default()
		}
}
pub fn get_idle_thread() -> ::threads::ThreadPtr {
	todo!("get_idle_thread");
}

pub fn set_thread_ptr(thread: ::threads::ThreadPtr) {
	let real = borrow_thread_mut();
	if real.is_null() {
		// SAFE: Valid ASM
		unsafe { asm!("mcr p15,0, $0, c13,c0,4" : : "r" (thread.into_usize())); }
	}
	else if real as *const _ == &*thread {
		// Convert and discard
		thread.into_usize();
	}
	else {
		panic!("");
	}
}
pub fn get_thread_ptr() -> Option<::threads::ThreadPtr> {
	// SAFE: Thread pointer should either be valid, or NULL
	unsafe {
		let cur: usize;
		asm!("mrc p15,0, $0, c13,c0,4" : "=r" (cur));
		if cur == 0 {
			None
		}
		else {
			Some( ::threads::ThreadPtr::from_usize(cur) )
		}
	}
}
fn borrow_thread_mut() -> *mut ::threads::Thread {
	// SAFE: Read-only access to the thread-local word
	unsafe {
		let ptr: usize;
		asm!("mrc p15,0, $0, c13,c0,4" : "=r" (ptr));
		(ptr & !1) as *mut _
	}
}
pub fn borrow_thread() -> *const ::threads::Thread {
	borrow_thread_mut() as *const _
}
pub fn switch_to(thread: ::threads::ThreadPtr) {
	#[allow(improper_ctypes)]
	extern "C" {
		fn task_switch(old_sp: &mut usize, new_sp: usize, ttbr0: u32, thread_ptr: usize);
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
	log_debug!("stack.pos() = {:#x}", stack.pos());
	let a = stack.pos();
	stack.align(8);
	stack.push(a);
	
	// 3. Populate with task_switch state
	// - Root function defined below
	stack.push( thread_root::<F> as usize );
	// - LR popped by task_switch - Trampoline that sets R0 to the address of 'code'
	stack.push( thread_trampoline as usize );
	// - R4-R12 saved by task_switch
	for _ in 4 .. 12+1 {
		stack.push(0u32);
	}
	stack.push(0u32);	// User SP
	stack.push(0u32);	// User LR
	
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

