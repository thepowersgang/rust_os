// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads/mod.rs
//! Thread management
use prelude::*;

mod thread;
mod thread_list;
mod wait_queue;

mod worker_thread;

mod sleep_object;

pub use self::thread::{Thread,ThreadHandle};

pub use self::thread_list::{ThreadList,THREADLIST_INIT};
pub use self::sleep_object::{SleepObject,SleepObjectRef};
pub use self::wait_queue::{WaitQueue,WAITQUEUE_INIT};

/// A bitset of wait events
pub type EventMask = u32;

///// A borrowed Box<Thread>, released when borrow expires
//struct BorrowedThread(Option<Box<Thread>>);

// ----------------------------------------------
// Statics
//static s_all_threads:	::sync::Mutex<Map<uint,*const Thread>> = mutex_init!(Map{});
#[allow(non_upper_case_globals)]
static s_runnable_threads: ::sync::Spinlock<ThreadList> = ::sync::Spinlock::new(THREADLIST_INIT);

// ----------------------------------------------
// Code
/// Initialise the threading subsystem
pub fn init()
{
	let mut tid0 = Thread::new_boxed(0, "ThreadZero");
	tid0.cpu_state = ::arch::threads::init_tid0_state();
	::arch::threads::set_thread_ptr( tid0 )
}

/// Yield control of the CPU for a short period (while polling or main thread halted)
pub fn yield_time()
{
	s_runnable_threads.lock().push( get_cur_thread() );
	reschedule();
}

pub fn yield_to(thread: Box<Thread>)
{
	log_debug!("Yielding CPU to {:?}", thread);
	s_runnable_threads.lock().push( get_cur_thread() );
	::arch::threads::switch_to( thread );
}

pub fn get_thread_id() -> thread::ThreadID
{
	let p = ::arch::threads::borrow_thread();
	if p == 0 as *const _ {
		0
	}
	else {
		unsafe { (*p).get_tid() }
	}
}

// TODO: Use a wrapper instead of &T
// TODO: Prevent this pointer from being sent (which will prevent accessing of freed memory)
pub fn get_process_local<T: Send+Sync>() -> &'static T
{
	todo!("get_process_local<{}>()", type_name!(T));
}

/// Pick a new thread to run and run it
///
/// NOTE: This can lead to the current thread being forgotten
#[doc(hidden)]
pub fn reschedule()
{
	loop
	{
		if let Some(thread) = get_thread_to_run()
		{
			if &*thread as *const _ == ::arch::threads::borrow_thread() as *const _
			{
				log_debug!("Task switch to self, idle");
				::arch::threads::switch_to(thread);
				::arch::threads::idle();
			}
			else
			{
				log_debug!("Task switch to {:?}", thread);
				::arch::threads::switch_to(thread);
				log_debug!("Awoke");
			}
			return ;
		}
		
		log_trace!("reschedule() - No active threads, idling");
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
//fn borrow_cur_thread() -> BorrowedThread
//{
//	BorrowedThread( Some(get_cur_thread()) )
//}

fn get_thread_to_run() -> Option<Box<Thread>>
{
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

//impl BorrowedThread
//{
//	fn take(mut self) -> Box<Thread> {
//		self.0.take().unwrap()
//	}
//}
//impl Drop for BorrowedThread
//{
//	fn drop(&mut self) {
//		match self.0.take()
//		{
//		Some(v) => rel_cur_thread(v),
//		None => {},
//		}
//	}
//}
//impl ::core::ops::Deref for BorrowedThread
//{
//	type Target = Thread;
//	fn deref(&self) -> &Thread { &**self.0.as_ref().unwrap() }
//}

// vim: ft=rust

