// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/threads/thread.rs
//! Representation of an active thread
/**
 * Ownership
 * =========
 *
 * The `Thread` struct is owned by the thread itself (the pointer stored within TLS)
 * however, it points to a shared block that contains information needed by both the 
 * thread itself, and the "owner" of the thread (e.g process, or controlling driver).
 */
use prelude::*;
use lib::mem::Arc;

/// Thread identifier (unique)
pub type ThreadID = u32;
pub type ProcessID = u32;

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
	/// Dead, waiting to be reaped
	Dead(u32),
}
// Sendable, the objects it points to must be either boxed or 'static
unsafe impl Send for RunState { }
impl Default for RunState { fn default() -> RunState { RunState::Runnable } }

pub struct Process
{
	name: String,
	pid: ProcessID,
	address_space: ::memory::virt::AddressSpace,
	// TODO: use of a tuple here looks a little crufty
	exit_status: ::sync::Mutex< (Option<u32>, Option<::threads::sleep_object::SleepObjectRef>) >,
	pub proc_local_data: ::sync::RwLock<Vec< ::lib::mem::aref::Aref<::core::any::Any+Sync+Send> >>,
}
/// Handle to a process, used for spawning and communicating
pub struct ProcessHandle(Arc<Process>);
impl_fmt! {
	Debug(self, f) for ProcessHandle {
		write!(f, "P({} {})", self.0.pid, self.0.name)
	}
}

struct SharedBlock
{
	name: String,
	tid: ThreadID,
	process: Arc<Process>,
}

/// An owning thread handle
pub struct ThreadHandle
{
	block: Arc<SharedBlock>,
	// TODO: Also store a pointer to the 'Thread' struct?
	// - Race problems
}

/// Thread information
pub struct Thread
{
	block: Arc<SharedBlock>,
	/// Execution state
	pub run_state: RunState,
	
	/// CPU state
	pub cpu_state: ::arch::threads::State,
	/// Next thread in intrusive list
	pub next: Option<Box<Thread>>,
}
assert_trait!{Thread : Send}

/// Last allocated TID (because TID0 is allocated differently)
static S_LAST_TID: ::core::sync::atomic::AtomicUsize = ::core::sync::atomic::ATOMIC_USIZE_INIT;
const C_MAX_TID: usize = 0x7FFF_FFF0;	// Leave 16 TIDs spare at end of 31 bit number
static S_LAST_PID: ::core::sync::atomic::AtomicUsize = ::core::sync::atomic::ATOMIC_USIZE_INIT;
const C_MAX_PID: usize = 0x007F_FFF0;	// Leave 16 PIDs spare at end of 23 bit number

fn allocate_tid() -> ThreadID
{
	// Preemptively prevent rollover
	if S_LAST_TID.load(::core::sync::atomic::Ordering::Relaxed) == C_MAX_TID - 1 {
		panic!("TODO: Handle TID exhaustion by searching for free");
	}
	let rv = S_LAST_TID.fetch_add(1, ::core::sync::atomic::Ordering::Relaxed);
	// Handle rollover after (in case of heavy contention)
	if rv >= C_MAX_TID {
		panic!("TODO: Handle TID exhaustion by searching for free (raced)");
	}
	
	(rv + 1) as ThreadID
}

fn allocate_pid() -> u32
{
	// Preemptively prevent rollover
	if S_LAST_PID.load(::core::sync::atomic::Ordering::Relaxed) == C_MAX_PID - 1 {
		panic!("TODO: Handle PID exhaustion by searching for free");
	}
	let rv = S_LAST_PID.fetch_add(1, ::core::sync::atomic::Ordering::Relaxed);
	// Handle rollover after (in case of heavy contention)
	if rv >= C_MAX_PID {
		panic!("TODO: Handle PID exhaustion by searching for free (raced)");
	}
	
	(rv + 1) as u32
}

impl Process
{
	pub fn new_pid0() -> Arc<Process> {
		Arc::new(Process {
			name: String::from("PID0"),
			pid: 0,
			exit_status: Default::default(),
			address_space: ::memory::virt::AddressSpace::pid0(),
			proc_local_data: ::sync::RwLock::new( Vec::new() ),
		})
	}
	pub fn new<S: Into<String>+::core::fmt::Debug>(name: S, addr_space: ::memory::virt::AddressSpace) -> Arc<Process>
	{
		Arc::new(Process {
			pid: allocate_pid(),
			name: name.into(),
			exit_status: Default::default(),
			address_space: addr_space,
			proc_local_data: ::sync::RwLock::new( Vec::new() ),
		})
	}
	
	fn empty_cpu_state(&self) -> ::arch::threads::State {
		::arch::threads::State::new( &self.address_space )
	}

	pub fn get_pid(&self) -> ProcessID { self.pid }

	pub fn mark_exit(&self, status: u32) -> Result<(),()> {
		let mut lh = self.exit_status.lock();
		if lh.0.is_some() {
			Err( () )
		}
		else {
			
			if let Some(ref sleep_ref) = lh.1 {
				sleep_ref.signal();
			}

			lh.0 = Some(status);
			Ok( () )
		}
	}
}
impl ProcessHandle
{
	pub fn new<S: Into<String>+::core::fmt::Debug>(name: S, clone_start: usize, clone_end: usize) -> ProcessHandle {
		ProcessHandle( Process::new(name, ::memory::virt::AddressSpace::new(clone_start, clone_end)) )
	}
	
	pub fn start_root_thread(&mut self, ip: usize, sp: usize) {
		assert!( ::lib::mem::arc::get_mut(&mut self.0).is_some() );
		
		let mut thread = Thread::new_boxed(allocate_tid(), format!("{}#1", self.0.name), self.0.clone());
		::arch::threads::start_thread( &mut thread,
			// SAFE: Well... trusting caller to give us sane addresses etc, but that's the user's problem
			move || unsafe {
					log_debug!("Dropping to {:#x} SP={:#x}", ip, sp);
					::arch::drop_to_user(ip, sp, 0)
				}
			);
		super::yield_to(thread);
	}

	pub fn get_process_local<T: Send+Sync+::core::marker::Reflect+Default+'static>(&self) -> Option<::lib::mem::aref::ArefBorrow<T>> {
		let pld = &self.0.proc_local_data;
		// 1. Try without write-locking
		for s in pld.read().iter()
		{
			let item_ref: &::core::any::Any = &**s;
			if item_ref.get_type_id() == ::core::any::TypeId::of::<T>() {
				return Some( s.borrow().downcast::<T>().ok().unwrap() );
			}
		}
		None
	}


	pub fn bind_wait_terminate(&self, obj: &mut ::threads::SleepObject) {
		let mut lh = self.0.exit_status.lock();
		if let Some(_status) = lh.0 {
			obj.signal();
		}
		else if let Some(_) = lh.1 {
			todo!("Multiple threads sleeping on this process");
		}
		else {
			lh.1 = Some( obj.get_ref() );
		}
	}
	pub fn clear_wait_terminate(&self, obj: &mut ::threads::SleepObject) -> bool {
		let mut lh = self.0.exit_status.lock();

		assert!(lh.1.is_some());
		assert!(lh.1.as_ref().unwrap().is_from(obj));
		lh.1 = None;
		
		lh.0.is_some()
	}

	pub fn get_exit_status(&self) -> Option<u32> {
		self.0.exit_status.lock().0
	}
}

impl ThreadHandle
{
	pub fn new<F: FnOnce()+Send+'static, S: Into<String>>(name: S, fcn: F, process: Arc<Process>) -> ThreadHandle
	{
		let mut thread = Thread::new_boxed(allocate_tid(), name, process);
		let handle = ThreadHandle {
			block: thread.block.clone(),
			};
		::arch::threads::start_thread(&mut thread, fcn);
		
		// Yield to this thread
		super::yield_to(thread);
		
		handle
	}
}
impl ::core::fmt::Debug for ThreadHandle
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> Result<(),::core::fmt::Error>
	{
		write!(f, "ThreadHandle({})", self.block)
	}
}
impl ::core::ops::Drop for ThreadHandle
{
	fn drop(&mut self) {
		panic!("TODO: Wait for thread to terminate when handle is dropped");
	}
}

impl Thread
{
	/// Create a new thread
	pub fn new_boxed<S: Into<String>>(tid: ThreadID, name: S, process: Arc<Process>) -> Box<Thread>
	{
		let rv = box Thread {
			cpu_state: process.empty_cpu_state(),
			block: Arc::new( SharedBlock { tid: tid, name: name.into(), process: process } ),
			run_state: RunState::Runnable,
			next: None,
			};
		
		// TODO: Add to global list of threads (removed on destroy)
		log_debug!("Creating thread {:?}", rv);
		
		rv
	}
	
	pub fn get_tid(&self) -> ThreadID { self.block.tid }
	
	/// Set the execution state of this thread
	pub fn set_state(&mut self, state: RunState) {
		self.run_state = state;
	}
	
	pub fn is_runnable(&self) -> bool { is!(self.run_state, RunState::Runnable) }
	
	/// Assert that this thread is runnable
	pub fn assert_active(&self) {
		assert!( !is!(self.run_state, RunState::Sleep(_)) );
		assert!( !is!(self.run_state, RunState::ListWait(_)) );
		assert!( is!(self.run_state, RunState::Runnable) );
	}
	
	pub fn get_process_info(&self) -> &Process {
		&*self.block.process
	}
}

impl ::core::fmt::Display for SharedBlock
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		write!(f, "{} {}", self.tid, self.name)
	}
}

impl ::core::fmt::Debug for Thread
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> Result<(),::core::fmt::Error>
	{
		write!(f, "{:p}({})", self, self.block)
	}
}

impl_fmt! {
	Display(self, f) for Process {
		write!(f, "PID{}:'{}'", self.pid, self.name)
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

