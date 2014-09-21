//
//

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
	fn task_switch(oldrsp: &mut u64, newrsp: u64, cr3: u64, tlsbase: u64);
}
#[thread_local]
static mut t_thread_ptr: *mut () = 0 as *mut ();

pub fn init_tid0_state() -> State
{
	State {
		cr3: &low_InitialPML4 as *const _ as u64,
		rsp: 0,
		tlsbase: &TID0TLS as *const _ as u64,
		}
}

pub fn switch_to(state: &State, outstate: &mut State)
{
	unsafe
	{
		// TODO: Lazy save/restore SSE state
		task_switch(&mut outstate.rsp, state.rsp, state.cr3, state.tlsbase);
	}
}

pub fn get_thread_ptr() -> ::threads::ThreadHandle
{
	unsafe {
		assert!( t_thread_ptr as uint != 0 );
		::core::mem::transmute( t_thread_ptr )
	}
}
pub fn set_thread_ptr(ptr: ::threads::ThreadHandle)
{
	unsafe {
		assert!( t_thread_ptr as uint == 0 );
		t_thread_ptr = ::core::mem::transmute(ptr);
	}
}

// vim: ft=rust

