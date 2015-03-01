// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads/thread.rs
//! Representation of an active thread

use _common::*;

/// Thread identifier (unique)
pub type ThreadID = u32;
/// A thread handle
pub type ThreadHandle = Box<Thread>;

//#[deriving(PartialEq)]
/// Thread run state
pub enum RunState
{
	/// Runnable = Can be executed (either currently running, or on the active queue)
	Runnable,
	/// Sleeping on a WaitQueue
	ListWait(*const super::WaitQueue),
	/// Sleeping on a SleepObject
	Sleep(*const super::sleep_object::SleepObject),
	///// Dead, waiting to be reaped
	//Dead(u32),
}
impl Default for RunState { fn default() -> RunState { RunState::Runnable } }

/// Thread information
pub struct Thread
{
	name: String,
	tid: ThreadID,
	/// Execution state
	pub run_state: RunState,
	
	/// CPU state
	pub cpu_state: ::arch::threads::State,
	/// Next thread in intrusive list
	pub next: Option<Box<Thread>>,
}


impl Thread
{
	/// Create a new thread
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
		log_debug!("Creating thread {:?}", rv);
		
		rv
	}
	
	/// Set the name of this thread
	pub fn set_name(&mut self, name: String) {
		self.name = name;
	}
	/// Set the execution state of this thread
	pub fn set_state(&mut self, state: RunState) {
		self.run_state = state;
	}
	/// Assert that this thread is runnable
	pub fn assert_active(&self) {
		assert!( !is!(self.run_state, RunState::Sleep(_)) );
		assert!( !is!(self.run_state, RunState::ListWait(_)) );
		assert!( is!(self.run_state, RunState::Runnable) );
	}
}

impl ::core::fmt::Debug for Thread
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> Result<(),::core::fmt::Error>
	{
		write!(f, "{:p}({} {})", self as *const _, self.tid, self.name)
	}
}

impl ::core::ops::Drop for Thread
{
	fn drop(&mut self)
	{
		// TODO: Remove self from the global thread map
		log_debug!("Destroying thread {:?}", self);
	}
}

