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
	Sleep(*const super::sleep_object::SleepObject<'static>),
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
	pub proc_local_data: ::sync::RwLock<Vec< ::lib::mem::aref::Aref<dyn core::any::Any+Sync+Send> >>,
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
	complete: crate::sync::EventChannel,
}

/// An owning thread handle
pub struct ThreadHandle
{
	block: Arc<SharedBlock>,
	// TODO: Also store a pointer to the 'Thread' struct?
	// - Race problems
}

/// "Owned" pointer to a thread (panics if dropped)
pub struct ThreadPtr(::lib::mem::Unique<Thread>);

/// Thread information
pub struct Thread
{
	block: Arc<SharedBlock>,
	/// Execution state
	pub run_state: RunState,
	
	/// CPU state
	pub cpu_state: ::arch::threads::State,
	/// Next thread in intrusive list
	pub next: Option<ThreadPtr>,
}
assert_trait!{Thread : Send}

/// Last allocated TID (because TID0 is allocated differently)
static S_LAST_TID: ::core::sync::atomic::AtomicUsize = ::core::sync::atomic::AtomicUsize::new(0);
const C_MAX_TID: usize = 0x7FFF_FFF0;	// Leave 16 TIDs spare at end of 31 bit number
static S_LAST_PID: ::core::sync::atomic::AtomicUsize = ::core::sync::atomic::AtomicUsize::new(0);
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
		ProcessHandle( Process::new(name, ::memory::virt::AddressSpace::new(clone_start, clone_end).expect("ProcessHandle::new - OOM")) )
	}
	
	pub fn start_root_thread(&mut self, ip: usize, sp: usize) {
		log_trace!("start_thread(self={:?}, ip={:#x}, sp={:#x})", self, ip, sp);
		assert!( Arc::get_mut(&mut self.0).is_some() );
		
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

	pub fn get_process_local<T>(&self) -> Option<::lib::mem::aref::ArefBorrow<T>>
	where
		T: Send+Sync+::core::any::Any+Default+'static
	{
		let pld = &self.0.proc_local_data;
		// 1. Try without write-locking
		for s in pld.read().iter()
		{
			let item_ref: &dyn core::any::Any = &**s;
			if item_ref.type_id() == ::core::any::TypeId::of::<T>() {
				return Some( s.borrow().downcast::<T>().ok().unwrap() );
			}
		}
		None
	}

	pub fn get_process_local_alloc<T>(&self) -> ::lib::mem::aref::ArefBorrow<T>
	where
		T: Send+Sync+::core::any::Any+Default+'static
	{
		let pld = &self.0.proc_local_data;
		// 1. Try without write-locking
		for s in pld.read().iter()
		{
			let item_ref: &dyn core::any::Any = &**s;
			if item_ref.type_id() == ::core::any::TypeId::of::<T>() {
				return s.borrow().downcast::<T>().ok().unwrap();
			}
		}
		// 2. Try _with_ write-locking
		let mut lh = pld.write();
		for s in lh.iter()
		{
			let item_ref: &dyn core::any::Any = &**s;
			if item_ref.type_id() == ::core::any::TypeId::of::<T>() {
				return s.borrow().downcast::<T>().ok().unwrap();
			}
		}
		// 3. Create an instance
		log_debug!("Creating instance of {} for {:?} (remote)", type_name!(T), self);
		let buf = ::lib::mem::aref::Aref::new(T::default());
		let ret = buf.borrow();
		lh.push( buf );
		ret
	}


	pub fn bind_wait_terminate(&self, obj: &mut ::threads::SleepObject) {
		log_trace!("bind_wait_terminate({:p}, obj={:p})", self, obj);
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
		log_trace!("clear_wait_terminate({:p}, obj={:p})", self, obj);
		let mut lh = self.0.exit_status.lock();

		if let Some(ref v) = lh.1 {
			assert!(v.is_from(obj), "clear_wait_terminate from different object");
		}
		else {
			log_trace!("- Wasn't registered");
		}
		lh.1 = None;
		
		lh.0.is_some()
	}

	pub fn get_exit_status(&self) -> Option<u32> {
		self.0.exit_status.lock().0
	}
}
impl ::core::ops::Drop for ProcessHandle {
	fn drop(&mut self) {
		log_notice!("Dropping handle {:?} - ref_count={}", self, Arc::strong_count(&self.0));
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
		let block = thread.block.clone();
		::arch::threads::start_thread(&mut thread, move || {
			fcn();
			block.complete.post();
			});
		
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
		super::yield_time();
		self.block.complete.sleep();
	}
}

impl ThreadPtr {
	pub fn new(ptr: Box<Thread>) -> ThreadPtr {
		// SAFE: Non-zero value
		ThreadPtr( unsafe { ::lib::mem::Unique::new_unchecked( Box::into_raw(ptr) ) } )
	}
	pub fn new_static(ptr: &'static mut Thread) -> ThreadPtr {
		// SAFE: Non-zero value
		ThreadPtr( unsafe { ::lib::mem::Unique::new_unchecked( (ptr as *mut _ as usize | 1) as *mut Thread) } )
	}
	pub fn into_boxed(self) -> Result<Box<Thread>, &'static mut Thread> {
		let p = self.0.as_ptr() as usize;
		::core::mem::forget(self);
		if p & 1 == 0 {
			// SAFE: bit 0 unset indicates heap pointer
			Ok( unsafe { Box::from_raw(p as *mut Thread) } )
		}
		else {
			// SAFE: bit 1 is cleared, pointer is valid
			Err( unsafe { &mut *( (p & !1) as *mut Thread ) } )
		}
	}
	fn as_ptr(&self) -> *mut Thread {
		let p = (self.0.as_ptr() as usize) & !1;
		p as *mut Thread
	}
	pub fn unwrap(self) -> *mut Thread {
		let rv = self.as_ptr();
		::core::mem::forget(self);
		rv
	}

	pub fn into_usize(self) -> usize {
		let rv = self.0.as_ptr() as usize;
		::core::mem::forget(self);
		rv
	}
	pub unsafe fn from_usize(v: usize) -> Self {
		ThreadPtr( ::lib::mem::Unique::new_unchecked( v as *mut Thread ) )
	}
}
impl ::core::ops::Deref for ThreadPtr {
	type Target = Thread;
	fn deref(&self) -> &Thread {
		// SAFE: Owned pointer
		unsafe { &*self.as_ptr() }
	}
}
impl ::core::ops::DerefMut for ThreadPtr {
	fn deref_mut(&mut self) -> &mut Thread {
		// SAFE: Owned pointer
		unsafe { &mut *self.as_ptr() }
	}
}
impl ::core::ops::Drop for ThreadPtr {
	fn drop(&mut self) {
		panic!("Dropping an owned thread pointer - {:?}", self);
	}
}
impl ::core::fmt::Debug for ThreadPtr {
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		let t: &Thread = &self;
		::core::fmt::Debug::fmt( t, f )
	}
}

impl Thread
{
	/// Create a new thread
	pub fn new_boxed<S: Into<String>>(tid: ThreadID, name: S, process: Arc<Process>) -> ThreadPtr
	{
		let rv = box Thread {
			cpu_state: process.empty_cpu_state(),
			block: Arc::new(SharedBlock {
				tid: tid,
				name: name.into(),
				process: process,
				complete: crate::sync::EventChannel::new(),
				}),
			run_state: RunState::Runnable,
			next: None,
			};
		
		// TODO: Add to global list of threads (removed on destroy)
		log_debug!("Creating thread {:?}", rv);
		
		ThreadPtr::new( rv )
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

pub fn new_idle_thread(cpu: usize) -> ThreadPtr {
	let mut thread = Thread::new_boxed(allocate_tid(), format!("Idle#{}", cpu), super::S_PID0.clone());
	::arch::threads::start_thread(&mut thread, super::idle_thread);
	thread
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
		log_debug!("Destroying thread {:?} - {} handles to block, {} to process", self, Arc::strong_count(&self.block), Arc::strong_count(&self.block.process));
	}
}

#[no_mangle]
/// Function called by `core-futures` to allow a generator to have access to the futures context
pub extern "C" fn set_tls_futures_context(p: *mut u8) -> *mut u8 {
	//log_debug!("set_tls_futures_context: {:?}", p);
	::arch::threads::set_tls_futures_context(p as *mut ()) as *mut u8
}


