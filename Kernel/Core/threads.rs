// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads.rs
// - Thread management
use _common::*;

use lib::mem::Rc;
use core::cell::RefCell;
use lib::Queue;

pub type ThreadHandle = Rc<RefCell<Thread>>;

//#[deriving(PartialEq)]
enum RunState
{
	StateRunnable,
	StateListWait(*const WaitQueue),
	StateEventWait(u32),
	StateDead(u32),
}
impl Default for RunState { fn default() -> RunState { StateRunnable } }

#[deriving(Default)]
struct Thread
{
	//name: String,
	tid: uint,
	run_state: RunState,
	
	cpu_state: ::arch::threads::State,
	next: Option<ThreadHandle>,
}

pub struct WaitQueue
{
	first: Option<ThreadHandle>,
	last: Option<ThreadHandle>
}
pub const WAITQUEUE_INIT: WaitQueue = WaitQueue {first: None, last: None};

// ----------------------------------------------
// Statics
//static s_all_threads:	::sync::Mutex<Map<ThreadHandle>> = mutex_init!("s_all_threads", Map{});
static mut s_runnable_threads: ::sync::Spinlock<Queue<ThreadHandle>> = spinlock_init!(queue_init!());

// ----------------------------------------------
// Code
pub fn init()
{
	let tid0 = Rc::new( RefCell::new(Thread {
		tid: 0,
		run_state: StateRunnable,
		cpu_state: ::arch::threads::init_tid0_state(),
		..Default::default()
		}) );
	::arch::threads::set_thread_ptr( tid0 )
}

pub fn reschedule()
{
	let cur = get_cur_thread();
	// 1. Get next thread
	log_trace!("reschedule()");
	let thread = get_thread_to_run();
	match thread
	{
	::core::option::None => {
		// Wait? How is there nothing to run?
		log_debug!("it's none");
		},
	::core::option::Some(t) => {
		// 2. Switch to next thread
		log_debug!("Task switch to {:u}", t.borrow().tid);
		if !t.is_same(&cur) {
			::arch::threads::switch_to(&t.borrow().cpu_state, &mut cur.borrow_mut().cpu_state);
		}
		}
	}
}

fn get_cur_thread() -> Rc<RefCell<Thread>>
{
	::arch::threads::get_thread_ptr()
}

fn get_thread_to_run() -> Option<Rc<RefCell<Thread>>>
{
	unsafe {
		let mut handle = s_runnable_threads.lock();
		let cur = get_cur_thread();
		if handle.empty()
		{
			if is!(cur.borrow().run_state, StateRunnable) {
				Some(cur)
			}
			else {
				None
			}
		}
		else
		{
			// 1. Put current thread on run queue (if needed)
			if is!(cur.borrow().run_state, StateRunnable) {
				log_trace!("Push current");
				handle.push(cur);
			}
			// 2. Pop off a new thread
			handle.pop()
		}
	}
}

impl WaitQueue
{
	pub fn wait<'a>(&mut self, lock_handle: ::arch::sync::HeldSpinlock<'a,bool>)
	{
		// 1. Lock global list?
		let cur = get_cur_thread();
		assert!(cur.borrow().next.is_none());
		// 2. Tack thread onto end
		if self.first.is_some()
		{
			assert!(self.last.is_some());
			let mut last_ref = self.last.as_ref().unwrap().borrow_mut();
			assert!(last_ref.next.is_none());
			last_ref.next = Some(cur);
		}
		else
		{
			assert!(self.last.is_none());
			self.first = Some(cur);
			self.last = Some(cur);
		}
		cur.borrow_mut().run_state = StateListWait(self as *mut _ as *const _);	// Keep rawptr kicking around for debug purposes
		// Unlock handle (short spinlocks disable interrupts)
		{ let _ = lock_handle; }
		// 4. Reschedule, and should return with state changed to run
		reschedule();
		assert!( !is!(cur.borrow().run_state, StateListWait(_)) );
		assert!( is!(cur.borrow().run_state, StateRunnable) );
	}
	pub fn wake_one(&mut self)
	{
		match self.first.take()
		{
		Some(t) => {
			self.first = t.borrow_mut().next.take();
			if self.first.is_none() {
				self.last = None;
			}
			},
		None => {},
		}
	}
}

// vim: ft=rust

