// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/arch/amd64/threads.rs
//! Architecture-level thread handling (helpers for ::threads).
use _common::*;

#[derive(Default)]//,Copy,Clone)]
/// Low-level thread state
pub struct State
{
	cr3: u64,
	rsp: u64,
	tlsbase: u64,
	// TODO: SSE state 
}

extern "C" {
	static low_InitialPML4: ();
	static v_ktls_size: ();
	static v_t_thread_ptr_ofs: ();
	static s_tid0_tls_base: u64;
	fn task_switch(oldrsp: &mut u64, newrsp: &u64, cr3: u64, tlsbase: u64);
}

#[thread_local]
#[no_mangle]
pub static mut t_thread_ptr: *mut ::threads::Thread = 0 as *mut _;
#[thread_local]
static mut t_thread_ptr_sent: bool = false;
//#[attribute(address_space(256))]
//static cpu_switch_disable: AtomicUsize = ATOMIC_USIZE_INIT;

/// Returns the thread state for TID0 (aka the kernel's core thread)
pub fn init_tid0_state() -> State
{
	State {
		cr3: &low_InitialPML4 as *const _ as u64,
		rsp: 0,
		tlsbase: s_tid0_tls_base,
		}
}

/// Idle for a short period, called when the CPU has nothing else to do
pub fn idle()
{
	if true {
		let flags = unsafe { let v: u64; asm!("pushf; pop $0" : "=r" (v)); v };
		assert!(flags & 0x200 != 0, "idle() with IF clear, RFLAGS = {:#x}", flags);
	}
	unsafe { asm!("hlt" : : : : "volatile"); }
}

/// Prepares the TLS block at the stop of a kernel stack
#[no_stack_check]
#[no_mangle]
pub extern "C" fn prep_tls(top: usize, bottom: usize, thread_ptr: *mut ::threads::Thread) -> usize
{
	/*const*/ let tls_size = &v_ktls_size as *const () as usize;
	/*const*/ let t_thread_ptr_ofs = &v_t_thread_ptr_ofs as *const () as usize;
	
	let mut pos = top;
	
	let tlsblock_top = pos;
	pos -= tls_size;
	let tlsblock = pos;
	// - Populate the TLS data area from the template
	unsafe {
		::lib::mem::memset(tlsblock as *mut u8, 0, tls_size);
		::core::ptr::write( (tlsblock + t_thread_ptr_ofs) as *mut *mut ::threads::Thread, thread_ptr );
	}
	
	#[repr(C)]
	struct TlsPtrArea
	{
		data_ptr: usize,
		_unk: [u64; 13],
		stack_limit: usize, 
	}
	pos -= ::core::mem::size_of::<TlsPtrArea>();
	pos -= pos % ::core::mem::min_align_of::<TlsPtrArea>();
	let tls_base = pos;
	unsafe {
		let tls_base = &mut *(tls_base as *mut TlsPtrArea);
		tls_base.data_ptr = tlsblock_top;
		tls_base.stack_limit = bottom;
	}
	
	tls_base
}

/// Start a new thread using the provided TCB
///
/// Allocates a new stack within the current address space
pub fn start_thread<F: FnOnce()+Send>(mut thread: Box<::threads::Thread>, code: F)
{
	let stack = ::memory::virt::alloc_stack().into_array::<u8>();;
	
	let stack_rgn_top = &stack[stack.len()-1] as *const _ as usize + 1;
	let mut stack_top = stack_rgn_top;
	let stack_bottom = &stack[0] as *const _ as usize;
	
	// 1. Allocate TLS block at the top of the stack
	log_trace!("prep_tls({:#x},{:#x},{:p})", stack_top, stack_bottom, &*thread);
	let tls_base = prep_tls(stack_top, stack_bottom, &mut *thread as *mut _);
	stack_top = tls_base;
	
	// 2. Populate stack with `code`
	stack_top -= ::core::mem::size_of::<F>();
	stack_top -= stack_top % ::core::mem::min_align_of::<F>();
	let code_ptr = stack_top;
	unsafe {
		::core::ptr::write(code_ptr as *mut F, code);
	}

	extern "C" {
		/// Pops the root function, and sets RDI=RSP
		fn thread_trampoline();
	}
	fn thread_root<F: FnOnce()+Send>(code_ptr: *const F) -> ! {
		// Copy the closure locally
		// - TODO: Find a way that avoids needing to make this unnessesary copy. By-value FnOnce is kinda undefined, sadly
		let code = unsafe { ::core::ptr::read(code_ptr) };
		// 1. Run closure
		code();
		// 2. terminate thread
		panic!("TODO: Terminate thread at end of thread_root");
	}
	
	// 3. Populate stack with trampoline state
	// - All that is needed is to push the trampoline address (it handles calling the rust code)
	unsafe {
		stack_top -= 8;
		::core::ptr::write(stack_top as *mut u64, thread_root::<F> as usize as u64);
		stack_top -= 8;
		::core::ptr::write(stack_top as *mut u64, thread_trampoline as usize as u64);
	}
	
	unsafe {
		//::logging::hex_dump( ::core::slice::from_raw_parts(stack_top as *const u8, stack_rgn_top - stack_top) );
		log_debug!("- stack_top = {:#x}, size = {}", stack_top, stack_rgn_top - stack_top);
		log_debug!("- stack = {:?}", ::logging::HexDump(&*(stack_top as *const [u8; 160])));
		log_debug!("- stack = [{:x}]", &(*(stack_top as *const [u64; 160/8]))[..]);
	}
	
	// 4. Apply newly updated state
	thread.cpu_state.rsp = stack_top as u64;
	thread.cpu_state.tlsbase = tls_base as u64;
	thread.cpu_state.cr3 = unsafe { (*t_thread_ptr).cpu_state.cr3 };
	
	// 5. Switch to new thread
	switch_to(thread);
}

/// Switch to the passed thread (suspending the current thread until it is rescheduled)
pub fn switch_to(newthread: Box<::threads::Thread>)
{
	if is_task_switching_disabled()
	{
		// This code should only ever trigger if a Spinlock-holding thread
		// was interrupted by an IRQ. If said thread attempts to sleep, it's an error
		assert!( newthread.is_runnable() );
	}
	else
	{
		unsafe
		{
			// TODO: Lazy save/restore SSE state
			let outstate = &mut (*t_thread_ptr).cpu_state;
			let state = &newthread.cpu_state;
			//assert!(state.rsp != 0);
			assert!(state.cr3 != 0);
			assert!(state.tlsbase != 0);
			task_switch(&mut outstate.rsp, &state.rsp, state.cr3, state.tlsbase);
		}
		unsafe
		{
			t_thread_ptr_sent = false;
			::core::mem::forget(newthread);
		}
	}
}

/// Obtain the current thread's pointer (as a owned box, thread is destroyed when box is dropped)
pub fn get_thread_ptr() -> Option<Box<::threads::Thread>>
{
	unsafe {
		assert!( !t_thread_ptr.is_null() );
		assert!( !t_thread_ptr_sent );
		t_thread_ptr_sent = true;
		::core::mem::transmute( t_thread_ptr )
	}
}
/// Release or set the current thread pointer
pub fn set_thread_ptr(ptr: Box<::threads::Thread>)
{
	unsafe {
		let ptr: *mut _ = ::core::mem::transmute(ptr);
		if t_thread_ptr == ptr {
			assert!( !t_thread_ptr_sent );
			t_thread_ptr_sent = false;
		}
		else {
			assert!( t_thread_ptr.is_null(), "t_thread_ptr is not null when set_thread_ptr is called, instead {:p}", t_thread_ptr );
			t_thread_ptr = ptr;
			t_thread_ptr_sent = false;
		}
	}
}

/// Disable task switching until corresponding `enable_task_switch` call
pub fn disable_task_switch()
{
	// TODO: increment CPU-local counter representing task switch state
}
/// Re-enable task switching
pub fn enable_task_switch()
{
}
/// Returns true is task switching is enabled
pub fn is_task_switching_disabled() -> bool
{
	// Return (s_cpu_local.
	false
}

// vim: ft=rust

