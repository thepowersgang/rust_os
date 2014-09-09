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

pub fn switch_to(state: &State)
{
	unsafe
	{
		// TODO: Lazy save/restore SSE state
		asm!(
			"push 1f"	// Save a return address
			"mov %0, %cr3"	// Switch address spaces
			"mov %1, %rsp"	// Switch stacks
			"ret"	// Jump to saved return address
			"1:"	// Target for completed switch
			:
			: "r" (state.cr3), "r" (state.rsp)
			: // TODO: List all callee save registers
			: "volatile"
			);
	}
}

pub fn get_thread_ptr() -> *mut ()
{
	0 as *mut ()
}
pub fn set_thread_ptr(ptr: *mut ())
{
	
}

// vim: ft=rust

