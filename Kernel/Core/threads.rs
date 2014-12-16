// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads.rs
// - Thread management
use _common::*;

pub type ThreadHandle = Box<Thread>;

//#[deriving(PartialEq)]
enum RunState
{
	Runnable,
	ListWait(*const WaitQueue),
	EventWait(u32),
	Dead(u32),
}
impl Default for RunState { fn default() -> RunState { RunState::Runnable } }

pub struct Thread
{
	name: String,
	tid: uint,
	run_state: RunState,
	
	pub cpu_state: ::arch::threads::State,
	next: Option<Box<Thread>>,
}

pub struct WaitQueue
{
	list: ThreadList,
}
struct ThreadList
{
	first: Option<Box<Thread>>,
	last: Option<*mut Thread>
}
const THREADLIST_INIT: ThreadList = ThreadList {first: None, last: None};
pub const WAITQUEUE_INIT: WaitQueue = WaitQueue { list: THREADLIST_INIT };

// ----------------------------------------------
// Statics
//static s_all_threads:	::sync::Mutex<Map<uint,*const Thread>> = mutex_init!(Map{});
#[allow(non_upper_case_globals)]
static s_runnable_threads: ::sync::Spinlock<ThreadList> = spinlock_init!(THREADLIST_INIT);

// ----------------------------------------------
// Code
pub fn init()
{
	let mut tid0 = Thread::new_boxed();
	tid0.name = String::from_str("ThreadZero");
	tid0.cpu_state = ::arch::threads::init_tid0_state();
	::arch::threads::set_thread_ptr( tid0 )
}

pub fn yield_time()
{
	s_runnable_threads.lock().push( get_cur_thread() );
	reschedule();
}

fn reschedule()
{
	// 1. Get next thread
	log_trace!("reschedule()");
	let thread = get_thread_to_run();
	match thread
	{
	None => {
		// Wait? How is there nothing to run?
		log_warning!("BUGCHECK: No runnable threads");
		},
	Some(t) => {
		// 2. Switch to next thread
		log_debug!("Task switch to {}", t);
		::arch::threads::switch_to(t);
		}
	}
}

fn get_cur_thread() -> Box<Thread>
{
	::arch::threads::get_thread_ptr().unwrap()
}
fn rel_cur_thread(t: Box<Thread>)
{
	::arch::threads::set_thread_ptr(t)
}

fn get_thread_to_run() -> Option<Box<Thread>>
{
	let mut handle = s_runnable_threads.lock();
	if handle.empty()
	{
		// WTF? At least an idle thread should be ready
		None
	}
	else
	{
		// 2. Pop off a new thread
		handle.pop()
	}
}

impl Thread
{
	pub fn new_boxed() -> Box<Thread>
	{
		let rv = box Thread {
			tid: 0,
			name: String::new(),
			run_state: RunState::Runnable,
			cpu_state: Default::default(),
			next: None,
			};
		
		// TODO: Add to global list of threads (removed on destroy)
		log_debug!("Creating thread {}", rv);
		
		rv
	}
}

impl ::core::fmt::Show for Thread
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> Result<(),::core::fmt::Error>
	{
		write!(f, "{}({} {})", self as *const _, self.tid, self.name)
	}
}

impl ::core::ops::Drop for Thread
{
	fn drop(&mut self)
	{
		// TODO: Remove self from the global thread map
		log_debug!("Destroying thread {}", self);
	}
}

impl ThreadList
{
	pub fn empty(&self) -> bool
	{
		self.first.is_none()
	}
	pub fn pop(&mut self) -> Option<Box<Thread>>
	{
		match self.first.take()
		{
		Some(mut t) => {
			self.first = t.next.take();
			if self.first.is_none() {
				self.last = None;
			}
			Some(t)
			},
		None => None
		}
	}
	pub fn push(&mut self, t: Box<Thread>)
	{
		assert!(t.next.is_none());
		// Save a pointer to the allocation
		let ptr = &*t as *const Thread as *mut Thread;
		log_debug!("Pushing thread {}", t);
		// 2. Tack thread onto end
		if self.first.is_some()
		{
			assert!(self.last.is_some());
			// Using unsafe and rawptr deref here is safe, because WaitQueue should be
			// locked (and nobody has any of the list items borrowed)
			unsafe {
				let last_ref = &mut *self.last.unwrap();
				assert!(last_ref.next.is_none());
				last_ref.next = Some(t);
			}
		}
		else
		{
			assert!(self.last.is_none());
			self.first = Some(t);
		}
		self.last = Some(ptr);
	}
}

impl WaitQueue
{
	pub fn wait<'a>(&mut self, lock_handle: ::arch::sync::HeldSpinlock<'a,bool>)
	{
		// 1. Lock global list?
		let mut cur = get_cur_thread();
		// - Keep rawptr kicking around for debug purposes
		cur.run_state = RunState::ListWait(self as *mut _ as *const _);
		// 2. Push current thread into waiting list
		self.list.push(cur);
		// 3. Unlock handle (short spinlocks disable interrupts)
		::core::mem::drop(lock_handle);
		// 4. Reschedule, and should return with state changed to run
		reschedule();
		
		let cur = get_cur_thread();
		assert!( !is!(cur.run_state, RunState::ListWait(_)) );
		assert!( is!(cur.run_state, RunState::Runnable) );
		rel_cur_thread(cur);
	}
	pub fn wake_one(&mut self)
	{
		match self.list.pop()
		{
		Some(mut t) => {
			t.run_state = RunState::Runnable;
			s_runnable_threads.lock().push(t);
			},
		None => {}
		}
	}
}

// vim: ft=rust

