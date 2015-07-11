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
// Sendable, the objects it points to must be either boxed or 'static
unsafe impl Send for RunState { }
impl Default for RunState { fn default() -> RunState { RunState::Runnable } }

pub struct Process
{
	name: String,
	pid: u32,
	address_space: ::memory::virt::AddressSpace,
	pub proc_local_data: ::sync::RwLock<Vec< ::lib::mem::aref::Aref<::core::any::Any+Sync+Send> >>,
}
/// Handle to a process, used for spawning and communicating
pub struct ProcessHandle(Arc<Process>);

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
static S_LAST_TID: ::core::atomic::AtomicUsize = ::core::atomic::ATOMIC_USIZE_INIT;
const C_MAX_TID: usize = 0x7FFF_FFF0;	// Leave 16 TIDs spare at end of 31 bit number
static S_LAST_PID: ::core::atomic::AtomicUsize = ::core::atomic::ATOMIC_USIZE_INIT;
const C_MAX_PID: usize = 0x007F_FFF0;	// Leave 16 TIDs spare at end of 23 bit number

fn allocate_tid() -> ThreadID
{
	// Preemptively prevent rollover
	if S_LAST_TID.load(::core::atomic::Ordering::Relaxed) == C_MAX_TID - 1 {
		panic!("TODO: Handle TID exhaustion by searching for free");
	}
	let rv = S_LAST_TID.fetch_add(1, ::core::atomic::Ordering::Relaxed);
	// Handle rollover after (in case of heavy contention)
	if rv >= C_MAX_TID {
		panic!("TODO: Handle TID exhaustion by searching for free (raced)");
	}
	
	(rv + 1) as ThreadID
}

fn allocate_pid() -> u32
{
	// Preemptively prevent rollover
	if S_LAST_PID.load(::core::atomic::Ordering::Relaxed) == C_MAX_PID - 1 {
		panic!("TODO: Handle PID exhaustion by searching for free");
	}
	let rv = S_LAST_PID.fetch_add(1, ::core::atomic::Ordering::Relaxed);
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
			address_space: ::memory::virt::AddressSpace::pid0(),
			proc_local_data: ::sync::RwLock::new( Vec::new() ),
		})
	}
	pub fn new<S: Into<String>+::core::fmt::Debug>(name: S) -> Arc<Process>
	{
		Arc::new(Process {
			pid: allocate_pid(),
			name: name.into(),
			address_space: ::memory::virt::AddressSpace::new(),
			proc_local_data: ::sync::RwLock::new( Vec::new() ),
		})
	}
}
impl ProcessHandle
{
	pub fn new<S: Into<String>+::core::fmt::Debug>(name: S) -> ProcessHandle {
		ProcessHandle( Process::new(name) )
	}
	
	/// Clone (COW) a portion of the current process's address space into this new process
	pub fn clone_from_cur(&mut self, dst_addr: usize, src_addr: usize, bytes: usize) {
		log_trace!("clone_from_cur( {:#x}, {:#x}+{:#x}", dst_addr, src_addr, bytes);
		if let Some(p) = ::lib::mem::arc::get_mut(&mut self.0) {
			p.address_space.clone_from_cur(dst_addr, src_addr, bytes)
		}
		else {
			panic!("Calling 'ProcessHandle::clone_from_cur' after first thread spawned");
		}
	}
	pub fn start_root_thread(&mut self, ip: usize, sp: usize) {
		assert!( ::lib::mem::arc::get_mut(&mut self.0).is_some() );
		todo!("ProcessHandle::start_root_thread");
	}
}

impl ThreadHandle
{
	pub fn new<F: FnOnce()+Send+'static>(name: &str, fcn: F, process: Arc<Process>) -> ThreadHandle
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
	pub fn new_boxed(tid: ThreadID, name: &str, process: Arc<Process>) -> Box<Thread>
	{
		let rv = box Thread {
			block: Arc::new( SharedBlock { tid: tid, name: From::from(name), process: process } ),
			run_state: RunState::Runnable,
			cpu_state: Default::default(),
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

