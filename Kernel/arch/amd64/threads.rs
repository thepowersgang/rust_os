//
//
use core::default::Default;

#[deriving(Default)]
pub struct State
{
	cr3: u64,
	rsp: u64,
	// TODO: SSE state 
}

extern "C" {
	static low_InitialPML4: ();
}
//pub static TID0STATE: State = State { cr3: &low_InitialPML4 as *const _ as u64, rsp: 0 };
pub static TID0STATE: State = State { cr3: 0x13d000, rsp: 0 };
#[thread_local]
static mut t_thread_ptr: *mut () = 0 as *mut ();

pub fn switch_to(state: &State, outstate: &mut State)
{
	unsafe
	{
		// TODO: Lazy save/restore SSE state
		asm!(concat!("push 1f\n",	// Save a return address
			"mov %rsp, ($0)\n",	// Save RSP
			"mov $1, %cr3\n",	// Switch address spaces
			"mov $2, %rsp\n",	// Switch stacks
			"ret\n",	// Jump to saved return address
			"1:\n",	// Target for completed switch
			"")
			: 
			: "r" (&mut outstate.rsp), "r" (state.cr3), "r" (state.rsp)
			: // TODO: List all callee save registers
			: "volatile"
			);
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

