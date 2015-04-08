// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads.rs
//! Thread management
use _common::*;

mod thread;
mod sleep_object;

pub use self::thread::Thread;
use self::thread::RunState;

pub use self::sleep_object::{SleepObject,SleepObjectRef};

/// A bitset of wait events
pub type EventMask = u32;

/// A borrowed Box<Thread>, released when borrow expires
struct BorrowedThread(Option<Box<Thread>>);

/// A list of waiting threads, can be woken one at a time, or all at once
pub struct WaitQueue
{
	list: ThreadList,
}

/// Intrusive linked list of threads
struct ThreadList
{
	first: Option<Box<Thread>>,
	last: Option<*mut Thread>
}
unsafe impl Send for ThreadList {}

const THREADLIST_INIT: ThreadList = ThreadList {first: None, last: None};
#[doc(hidden)]
pub const WAITQUEUE_INIT: WaitQueue = WaitQueue { list: THREADLIST_INIT };

// ----------------------------------------------
// Statics
//static s_all_threads:	::sync::Mutex<Map<uint,*const Thread>> = mutex_init!(Map{});
#[allow(non_upper_case_globals)]
static s_runnable_threads: ::sync::Spinlock<ThreadList> = spinlock_init!(THREADLIST_INIT);

// ----------------------------------------------
// Code
/// Initialise the threading subsystem
pub fn init()
{
	let mut tid0 = Thread::new_boxed();
	tid0.set_name( String::from_str("ThreadZero") );
	tid0.cpu_state = ::arch::threads::init_tid0_state();
	::arch::threads::set_thread_ptr( tid0 )
}

/// Yield control of the CPU for a short period (while polling or main thread halted)
pub fn yield_time()
{
	s_runnable_threads.lock().push( get_cur_thread() );
	reschedule();
	::arch::threads::idle();
}

/// Pick a new thread to run and run it
fn reschedule()
{
	// 1. Get next thread
	loop
	{
		if let Some(thread) = get_thread_to_run()
		{
			log_debug!("Task switch to {:?}", thread);
			::arch::threads::switch_to(thread);
			return ;
		}
		
		log_trace!("reschedule() - Idling");
		::arch::threads::idle();
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
fn borrow_cur_thread() -> BorrowedThread
{
	BorrowedThread( Some(get_cur_thread()) )
}

fn get_thread_to_run() -> Option<Box<Thread>>
{
	log_trace!("get_thread_to_run()");
        let _irq_lock = ::arch::sync::hold_interrupts();
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
		//log_debug!("Pushing thread {:?}", t);
		assert!(t.next.is_none());
		// Save a pointer to the allocation (for list tail)
		let ptr = &*t as *const Thread as *mut Thread;
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

impl Drop for BorrowedThread
{
	fn drop(&mut self) {
		rel_cur_thread(self.0.take().unwrap())
	}
}
impl ::core::ops::Deref for BorrowedThread
{
	type Target = Thread;
	fn deref(&self) -> &Thread { &**self.0.as_ref().unwrap() }
}

impl WaitQueue
{
	pub fn new() -> WaitQueue {
		WAITQUEUE_INIT
	}
	
	/// Wait the current thread on this queue, releasng the passed lock before actually sleeping
	// TODO: Rewrite such that HeldSpinlock<WaitQueue> can be passed in?
	pub fn wait<'a,T:Send>(&mut self, lock_handle: ::arch::sync::HeldSpinlock<'a,T>)
	{
		log_trace!("WaitQueue::wait(...)");
		// - Prevent interrupts from firing while we mess with the thread
		let _irq_lock = ::arch::sync::hold_interrupts();
		
		// 1. Lock global list?
		let mut cur = get_cur_thread();
		// - Keep rawptr kicking around for debug purposes
		cur.set_state( RunState::ListWait(self as *mut _ as *const _) );
		// 2. Push current thread into waiting list
		self.list.push(cur);
		// 3. Unlock protector, and allow IRQs once more
		::core::mem::drop(lock_handle);
		::core::mem::drop(_irq_lock);
		// 4. Reschedule, and should return with state changed to run
		reschedule();
		
		let cur = get_cur_thread();
		cur.assert_active();
		rel_cur_thread(cur);
	}
	/// Returns true if there is a thread waiting on the list
        pub fn has_waiter(&self) -> bool
        {
                ! self.list.empty()
        }
	/// Wake a single thread waiting on this queue
	pub fn wake_one(&mut self)
	{
		log_trace!("WaitQueue::wake_one()");
		match self.list.pop()
		{
		Some(mut t) => {
			t.set_state( RunState::Runnable );
                        let _irq_lock = ::arch::sync::hold_interrupts();
			s_runnable_threads.lock().push(t);
			},
		None => {}
		}
	}
}

// vim: ft=rust

