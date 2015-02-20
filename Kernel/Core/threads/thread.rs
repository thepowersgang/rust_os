
use _common::*;

pub type ThreadID = u32;
pub type ThreadHandle = Box<Thread>;

//#[deriving(PartialEq)]
pub enum RunState
{
	Runnable,
	ListWait(*const super::WaitQueue),
	Sleep(*const super::sleep_object::SleepObject),
	//Dead(u32),
}
impl Default for RunState { fn default() -> RunState { RunState::Runnable } }

pub struct Thread
{
	name: String,
	tid: ThreadID,
	pub run_state: RunState,
	
	pub cpu_state: ::arch::threads::State,
	pub next: Option<Box<Thread>>,
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
		log_debug!("Creating thread {:?}", rv);
		
		rv
	}
	
	pub fn set_name(&mut self, name: String) {
		self.name = name;
	}
	pub fn set_state(&mut self, state: RunState) {
		self.run_state = state;
	}
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

