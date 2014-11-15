//
//
use core::option::Option;
use lib::mem::Box;

#[deriving(Default)]
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

pub fn init_tid0_state() -> State
{
	State {
		cr3: &low_InitialPML4 as *const _ as u64,
		rsp: 0,
		tlsbase: &TID0TLS as *const _ as u64,
		}
}

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

pub fn get_thread_ptr() -> Option<Box<::threads::Thread>>
{
	unsafe {
		assert!( t_thread_ptr as uint != 0 );
		assert!( !t_thread_ptr_sent );
		t_thread_ptr_sent = true;
		::core::mem::transmute( t_thread_ptr )
	}
}
pub fn set_thread_ptr(ptr: Box<::threads::Thread>)
{
	unsafe {
		if t_thread_ptr as *const _ == &*ptr as *const _ {
			assert!( !t_thread_ptr_sent );
			t_thread_ptr_sent = false;
		}
		else {
			assert!( t_thread_ptr as uint == 0 );
			t_thread_ptr = ::core::mem::transmute(ptr);
			log_debug!("set_thread_ptr: t_thread_ptr = {}", t_thread_ptr);
			t_thread_ptr_sent = false;
		}
		
		log_debug!("set_thread_ptr: t_thread_ptr = {}, t_thread_ptr_sent = {}", t_thread_ptr, t_thread_ptr_sent);
	}
}

// vim: ft=rust

