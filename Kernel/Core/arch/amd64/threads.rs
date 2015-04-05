// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/arch/amd64/threads.rs
//! Architecture-level thread handling (helpers for ::threads).
use core::option::Option;
use lib::mem::Box;

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
	static TID0TLS: ();
	fn task_switch(oldrsp: &mut u64, newrsp: &u64, cr3: u64, tlsbase: u64);
}

#[thread_local]
static mut t_thread_ptr: *mut ::threads::Thread = 0 as *mut _;
#[thread_local]
static mut t_thread_ptr_sent: bool = false;

/// Returns the thread state for TID0 (aka the kernel's core thread)
pub fn init_tid0_state() -> State
{
	State {
		cr3: &low_InitialPML4 as *const _ as u64,
		rsp: 0,
		tlsbase: &TID0TLS as *const _ as u64,
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

/// Switch to the passed thread (suspending the current thread until it is rescheduled)
pub fn switch_to(newthread: Box<::threads::Thread>)
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
			assert!( t_thread_ptr.is_null() );
			t_thread_ptr = ptr;
			t_thread_ptr_sent = false;
		}
	}
}

// vim: ft=rust

