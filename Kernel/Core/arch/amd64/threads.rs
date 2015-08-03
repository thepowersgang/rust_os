// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/arch/amd64/threads.rs
//! Architecture-level thread handling (helpers for ::threads).
use prelude::*;

#[derive(Default)]//,Copy,Clone)]
/// Low-level thread state
pub struct State
{
	cr3: u64,
	rsp: u64,
	tlsbase: u64,
	// Not strictly part of the CPU state, but it prevents this thread's stack from disappearing
	stack_handle: Option< ::memory::virt::ArrayHandle<u8> >,
	// TODO: SSE state 
	// TODO: Usermode TLS bsae
}

extern "C" {
	static InitialPML4: ();
	static s_tid0_tls_base: u64;
	fn task_switch(oldrsp: &mut u64, newrsp: &u64, tlsbase: u64, cr3: u64);
}

#[repr(C)]
struct TLSData {
	// MUST be first (assumption in get_tls_ptr)
	// - TODO: Is this the same value as stack_top?
	// > Yes it is, but meh
	self_ptr: *const TLSData,
	// MUST be second (assumption in SYSCALL handler)
	stack_top: *const (),
	// MUST be third (same as above)
	user_stack: u64,
	
	// Free to reorder these
	thread_ptr: *mut ::threads::Thread,
	thread_ptr_lent: bool,
}


/// Returns the thread state for TID0 (aka the kernel's core thread)
pub fn init_tid0_state() -> State
{
	let cr3 = &InitialPML4 as *const _ as u64 - super::memory::addresses::IDENT_START as u64;
	log_debug!("init_tid0_state - cr3 = {:#x}", cr3);
	State {
		cr3: cr3,
		rsp: 0,
		tlsbase: s_tid0_tls_base,
		stack_handle: None,
		}
}

impl State
{
	/// Construct a new empty CPU state using the provided address space
	pub fn new(address_space: &::memory::virt::AddressSpace) -> State {
		log_trace!("State::new({:?})", address_space);
		let mut rv = State::default();
		rv.cr3 = address_space.inner().get_cr3();
		rv
	}
}

/// Idle for a short period, called when the CPU has nothing else to do
pub fn idle()
{
	if true {
		// SAFE: Just pulls rflags
		let flags = unsafe { let v: u64; asm!("pushf; pop $0" : "=r" (v)); v };
		assert!(flags & 0x200 != 0, "idle() with IF clear, RFLAGS = {:#x}", flags);
	}
	// SAFE: Safe assembly, just haldts
	unsafe { asm!("hlt" : : : : "volatile"); }
}

/// Prepares the TLS block at the stop of a kernel stack
#[no_mangle]
pub unsafe extern "C" fn prep_tls(top: usize, _bottom: usize, thread_ptr: *mut ::threads::Thread) -> usize
{
	let mut pos = top;
	
	// 1. Create the TLS data block
	pos -= ::core::mem::size_of::<TLSData>();
	let tlsblock = pos;
	
	// - Populate the TLS data area from the template
	//  > Also set the thread pointer
	let data_ptr = tlsblock as *mut TLSData;
	::core::ptr::write(data_ptr, TLSData {
		self_ptr: data_ptr,
		stack_top: tlsblock as *const (),
		user_stack: 0,
		
		thread_ptr: thread_ptr,
		thread_ptr_lent: false,
		});
	
	tlsblock
}

/// Start a new thread using the provided TCB
///
/// Allocates a new stack within the current address space
pub fn start_thread<F: FnOnce()+Send>(thread: &mut ::threads::Thread, code: F)
{
	let stack = ::memory::virt::alloc_stack().into_array::<u8>();
	
	let stack_rgn_top = &stack[stack.len()-1] as *const _ as usize + 1;
	let mut stack_top = stack_rgn_top;
	let stack_bottom = &stack[0] as *const _ as usize;
	
	// 1. Allocate TLS block at the top of the stack
	log_trace!("prep_tls({:#x},{:#x},{:p})", stack_top, stack_bottom, &*thread);
	// SAFE: Pointer is valid
	let tls_base = unsafe { prep_tls(stack_top, stack_bottom, thread as *mut _) };
	stack_top = tls_base;
	
	// 2. Populate stack with `code`
	stack_top -= ::core::mem::size_of::<F>();
	stack_top -= stack_top % ::core::mem::align_of::<F>();
	let code_ptr = stack_top;
	// SAFE: Pointer is valid
	unsafe {
		::core::ptr::write(code_ptr as *mut F, code);
	}
	
	// 3. Populate stack with trampoline state
	// - All that is needed is to push the trampoline address (it handles calling the rust code)
	// SAFE: Stack is valid for at least this many words (at least a page)
	unsafe {
		stack_top -= 8; ::core::ptr::write(stack_top as *mut u64, thread_root::<F> as usize as u64);
		// Trampoline that sets RDI to the address of 'code'
		stack_top -= 8; ::core::ptr::write(stack_top as *mut u64, thread_trampoline as usize as u64);
		// Six callee-save GPRs saved by task_switch
		stack_top -= 8; ::core::ptr::write(stack_top as *mut u64, 0xB4);
		stack_top -= 8; ::core::ptr::write(stack_top as *mut u64, 0xBB);
		stack_top -= 8; ::core::ptr::write(stack_top as *mut u64, 0x12);
		stack_top -= 8; ::core::ptr::write(stack_top as *mut u64, 0x13);
		stack_top -= 8; ::core::ptr::write(stack_top as *mut u64, 0x14);
		stack_top -= 8; ::core::ptr::write(stack_top as *mut u64, 0x15);
	}
	
	// 4. Apply newly updated state
	thread.cpu_state.rsp = stack_top as u64;
	thread.cpu_state.tlsbase = tls_base as u64;
	thread.cpu_state.stack_handle = Some(stack);

	// END: Parent function will run this thread for us
	
	extern "C" {
		/// Pops the root function, and sets RDI=RSP
		fn thread_trampoline();
	}
	fn thread_root<F: FnOnce()+Send>(code_ptr: *const F) -> ! {
		// Copy the closure locally
		// - TODO: Find a way that avoids needing to make this unnessesary copy. By-value FnOnce is kinda undefined, sadly
		// SAFE: Functionally owns that pointer
		let code = unsafe { ::core::ptr::read(code_ptr) };
		// 1. Run closure
		code();
		// 2. terminate thread
		panic!("TODO: Terminate thread at end of thread_root");
	}
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
			let outstate = &mut (*(*get_tls_ptr()).thread_ptr).cpu_state;
			let state = &newthread.cpu_state;
			// Don't assert RSP, could be switching to self
			assert!(state.cr3 != 0);
			assert!(state.tlsbase != 0);
			log_trace!("Switching to RSP={:#x},CR3={:#x},TLS={:#x}", state.rsp, state.cr3, state.tlsbase);
			assert!( *(outstate.tlsbase as *const usize) != 0, "outstate TLS Base clobbered before switch" );
			assert!( *(state.tlsbase as *const usize) != 0, "TLS Base clobbered before switch" );
			task_switch(&mut outstate.rsp, &state.rsp, state.tlsbase, state.cr3);
		}
		unsafe
		{
			(*get_tls_ptr()).thread_ptr_lent = false;
			::core::mem::forget(newthread);
		}
	}
}

fn get_tls_ptr() -> *mut TLSData {
	let ret;
	// SAFE: Just obtains the pointer from %gs
	unsafe { asm!("mov %gs:(0), $0" : "=r" (ret) ) }
	assert!(ret != 0 as *mut _);
	ret
}

/// Obtain the current thread's pointer (as a owned box, thread is destroyed when box is dropped)
pub fn get_thread_ptr() -> Option<Box<::threads::Thread>>
{
	// SAFE: Safe transmutes and derefs
	unsafe {
		let info = &mut *get_tls_ptr();
		assert!( !info.thread_ptr.is_null() );
		assert!( !info.thread_ptr_lent, "Thread {:?} already has its pointer lent", *info.thread_ptr );
		info.thread_ptr_lent = true;
		//log_debug!("Lend");
		::core::mem::transmute( info.thread_ptr )
	}
}
pub fn borrow_thread() -> *const ::threads::Thread {
	// SAFE: Safe dereference
	unsafe {
		(*get_tls_ptr()).thread_ptr
	}
}
/// Release or set the current thread pointer
pub fn set_thread_ptr(ptr: Box<::threads::Thread>)
{
	// SAFE: Good transmute/derefs
	unsafe {
		let ptr: *mut _ = ::core::mem::transmute(ptr);
		let info = &mut *get_tls_ptr();
		if info.thread_ptr == ptr {
			assert!( info.thread_ptr_lent, "Thread {:?}'s pointer received, but not lent", *info.thread_ptr );
		}
		else {
			assert!( info.thread_ptr.is_null(),
				"t_thread_ptr is not null when set_thread_ptr is called, instead {:p}",
				info.thread_ptr
				);
			info.thread_ptr = ptr;
		}
		//log_debug!("Receive");
		info.thread_ptr_lent = false;
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

