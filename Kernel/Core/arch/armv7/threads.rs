
use lib::mem::Box;
use threads::Thread;

#[derive(Default)]
pub struct State {
	sp: usize,
}

impl State
{
	pub fn new(address_space: &::memory::virt::AddressSpace) -> State {
		State::default()
	}
}

pub fn init_tid0_state() -> State {
	State::default()
}

pub fn set_thread_ptr(thread: Box<Thread>) {
}
pub fn get_thread_ptr() -> Option<Box<::threads::Thread>> {
	None
}
pub fn borrow_thread() -> *const ::threads::Thread {
	0 as *const _
}
pub fn switch_to(thread: Box<Thread>) {
}
pub fn idle() {
}

pub fn start_thread<F: FnOnce()+Send>(thread: &mut ::threads::Thread, code: F) {
}

