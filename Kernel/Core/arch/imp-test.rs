
pub mod memory {
	pub type PAddr = u64;
	pub type VAddr = usize;
	pub const PAGE_SIZE: usize = 4096;

	pub mod addresses {
		pub fn is_global(_addr: usize) -> bool {
			false
		}

		pub const STACK_SIZE: usize = 5*0x1000;

		pub const USER_END: usize = 0x8000_0000;

		pub const STACKS_BASE: usize = 0;
		pub const STACKS_END : usize = 0;

		pub const HARDWARE_BASE: usize = 0;
		pub const HARDWARE_END : usize = 0;

		pub const HEAP_START: usize = 0;
		pub const HEAP_END : usize = 0;

		pub const BUMP_START: usize = 0;
		pub const BUMP_END  : usize = 0;
	}
	pub mod virt {
		pub struct AddressSpace;
		impl AddressSpace
		{
			pub fn pid0() -> AddressSpace {
				AddressSpace
			}
			pub fn new(_cstart: usize, _cend: usize) -> Result<AddressSpace,()> {
				todo!("AddressSpace::new");
			}
		}

		pub fn post_init() {
		}

		pub unsafe fn temp_map<T>(_pa: super::PAddr)  -> *mut T {
			::core::ptr::null_mut()
		}
		pub unsafe fn temp_unmap<T>(_a: *mut T) {
		}

		pub fn get_phys<T>(_p: *const T) -> ::memory::PAddr {
			0
		}
		pub fn is_reserved<T>(_p: *const T) -> bool {
			false
		}
		pub fn get_info<T>(_p: *const T) -> Option<(::memory::PAddr,::memory::virt::ProtectionMode)> {
			None
		}

		pub fn is_fixed_alloc(_addr: *const (), _size: usize) -> bool {
			false
		}
		pub unsafe fn fixed_alloc(_p: ::memory::PAddr, _count: usize) -> Option<*mut ()> {
			None
		}

		pub fn can_map_without_alloc(_a: *mut ()) -> bool {
			false
		}

		pub unsafe fn map(_a: *mut (), _p: ::memory::PAddr, _mode: ::memory::virt::ProtectionMode) {
		}
		pub unsafe fn reprotect(_a: *mut (), _mode: ::memory::virt::ProtectionMode) {
		}
		pub unsafe fn unmap(_a: *mut ()) -> Option<::memory::PAddr> {
			None
		}
	}
	pub mod phys {
		pub fn ref_frame(_frame_idx: u64) {
		}
		pub fn deref_frame(_frame_idx: u64) -> u32 {
			1
		}
		pub fn get_multiref_count(_frame_idx: u64) -> u32 {
			0
		}

		pub fn mark_free(_frame_idx: u64) -> bool {
			false
		}
		pub fn mark_used(_frame_idx: u64) {
		}
	}
}
pub mod sync {
	use core::sync::atomic::Ordering;

	// TODO: use a mutex instead? (simulating a single-core machine really)
	#[derive(Default)]
	pub struct Spinlock<T>(::core::sync::atomic::AtomicBool, ::core::cell::UnsafeCell<T>);
	unsafe impl<T: Send> Sync for Spinlock<T> {}
	impl<T> Spinlock<T> {
		pub const fn new(v: T) -> Spinlock<T> {
			Spinlock( ::core::sync::atomic::AtomicBool::new(false), ::core::cell::UnsafeCell::new(v) )
		}
		pub fn get_mut(&mut self) -> &mut T {
			// SAFE: Mutable access
			unsafe { &mut *self.1.get() }
		}
		pub fn try_lock_cpu(&self) -> Option<HeldSpinlock<T>> {
			if self.0.compare_and_swap(false, true, Ordering::Acquire) {
				panic!("TODO: Spinlock::try_lock - already locked (need to check CPU)");
			}
			else {
				//println!("{:p} lock (try_lock_cpu)", self);
				Some(HeldSpinlock(self))
			}
		}
		pub fn lock(&self) -> HeldSpinlock<T> {
			if self.0.compare_and_swap(false, true, Ordering::Acquire) {
				panic!("TODO: Spinlock::lock - already locked");
			}
			else {
				//println!("{:p} lock", self);
				HeldSpinlock(self)
			}
		}
	}
	pub struct HeldSpinlock<'a, T: 'a>( &'a Spinlock<T> );
	impl<'a, T: 'a> ::core::ops::Deref for HeldSpinlock<'a, T> {
		type Target = T;
		fn deref(&self) -> &T {
			// SAFE: Locked
			unsafe { &*self.0 .1.get() }
		}
	}
	impl<'a, T: 'a> ::core::ops::DerefMut for HeldSpinlock<'a, T> {
		fn deref_mut(&mut self) -> &mut T {
			// SAFE: Locked
			unsafe { &mut *self.0 .1.get() }
		}
	}
	impl<'a, T: 'a> ::core::ops::Drop for HeldSpinlock<'a, T> {
		fn drop(&mut self) {
			assert!( self.0 .0.swap(false, Ordering::Release), "HeldSpinlock released on unlocked spinlock" );
			//println!("{:p} unlock", self.0);
		}
	}
	pub struct HeldInterrupts;

	pub fn hold_interrupts() -> HeldInterrupts {
		HeldInterrupts
	}

	pub unsafe fn stop_interrupts() {
	}
	pub unsafe fn start_interrupts() {
	}
}
pub mod interrupts {
	#[derive(Debug)]
	pub struct BindError;
	#[derive(Default)]
	pub struct IRQHandle;
	
	pub fn bind_gsi(_gsi: usize, _handler: fn(*const()), _info: *const ()) -> Result<IRQHandle, BindError> {
		todo!("bind_gsi")
	}
}
pub mod boot {
	pub fn get_boot_string() -> &'static str {
		""
	}
	pub fn get_video_mode() -> Option<::metadevs::video::bootvideo::VideoMode> {
		None
	}
	pub fn get_memory_map() -> &'static [::memory::MemoryMapEnt] {
		&[]
	}
}
pub mod pci {
	pub fn read(_a: u32) -> u32 {
		0
	}
	pub fn write(_a: u32, _v: u32) {
	}
}
pub mod threads {
	use std::sync::Arc;
	use std::cell::RefCell;
	use std::sync::atomic::{Ordering,AtomicBool};
	lazy_static::lazy_static! {
		static ref SWITCH_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new( () );
	}

	#[derive(Debug)]
	struct ThreadLocalState {
		ptr: *mut ::threads::Thread,
		ptr_moved: bool,
		this_state: Option<Arc<StateInner>>,
	}
	thread_local! {
		static THIS_THREAD_STATE: RefCell<ThreadLocalState> = RefCell::new(ThreadLocalState {
			ptr: std::ptr::null_mut(),
			ptr_moved: false,
			this_state: None,
			});
	}
	#[derive(Debug)]
	struct StateInner {
		condvar: std::sync::Condvar,
		complete: AtomicBool,
		running: AtomicBool,
	}
	pub struct State {
		thread_handle: Option<std::thread::JoinHandle<()>>,
		inner: Arc<StateInner>,
	}
	impl State
	{
		fn new_priv() -> State {
			State {
				thread_handle: None,
				inner: Arc::new(StateInner {
					condvar: Default::default(),
					complete: Default::default(),
					running: Default::default(),
				})
			}
		}
		pub fn new(_as: &::arch::memory::virt::AddressSpace) -> State {
			Self::new_priv()
		}
	}
	impl StateInner
	{
		fn sleep(&self, t: ::threads::ThreadPtr) {
			let lh = SWITCH_LOCK.lock().unwrap();
			t.cpu_state.inner.running.store(true, Ordering::SeqCst);	// Avoids a startup race
			t.cpu_state.inner.condvar.notify_one();
			if !self.complete.load(Ordering::SeqCst) {
				//log_trace!("{:p} sleeping", self);
				self.condvar.wait(lh).expect("Condvar wait failed");
			}
			else {
				//log_trace!("{:p} complete", self);
			}
			core::mem::forget(t);
		}
	}

	pub fn init_tid0_state() -> State {
		let rv = State::new_priv();
		let inner_handle = rv.inner.clone();
		log_trace!("init_tid0_state: {:p}", inner_handle);
		THIS_THREAD_STATE.with(|v| {
			let mut h = v.borrow_mut();
			assert!(h.this_state.is_none(), "TID0 alread initialised");
			h.this_state = Some(inner_handle);
			});
		rv
	}
	pub fn set_thread_ptr(t: ::threads::ThreadPtr) {
		THIS_THREAD_STATE.with(|v| {
			log_trace!("set_thread_ptr");
			let mut h = v.borrow_mut();
			let t: *mut _ = t.unwrap();
			if h.ptr.is_null() {
				h.ptr = t;
			}
			else {
				assert!(h.ptr == t);
				assert!(h.ptr_moved == true);
				h.ptr_moved = false;
			}
		})
	}
	pub fn get_thread_ptr() -> Option<::threads::ThreadPtr> {
		THIS_THREAD_STATE.with(|v| {
			log_trace!("get_thread_ptr: {:p}", v);
			let mut h = v.borrow_mut();
			assert!(!h.ptr_moved);
			if h.ptr.is_null() {
				None
			}
			else {
				h.ptr_moved = true;
				// SAFE: Pointer to pointer
				Some(unsafe { std::mem::transmute(h.ptr) })
			}
		})
	}
	pub fn borrow_thread() -> *const ::threads::Thread {
		THIS_THREAD_STATE.with(|v| {
			let h = v.borrow();
			// NOTE: Doesn't care if the pointer is "owned"
			h.ptr
		})
	}

	pub fn idle() {
		// Timed sleep?
	}
	pub fn get_idle_thread() -> ::threads::ThreadPtr {
		todo!("get_idle_thread");
	}
	pub fn switch_to(t: ::threads::ThreadPtr) {
		THIS_THREAD_STATE.with(|v| {
			let h = v.borrow();
			assert!( h.ptr_moved );
			match h.this_state
			{
			None => panic!("Current thread not initialised"),
			Some(ref v) => {
				log_trace!("switch_to: {:p} to {:p}", *v, t.cpu_state.inner);
				v.sleep(t);
				//log_trace!("switch_to: {:p} awake", *v);
				},
			}
		});
		THIS_THREAD_STATE.with(|v| {
			let mut h = v.borrow_mut();
			assert!(h.ptr_moved);
			h.ptr_moved = false;
		});
	}

	pub fn start_thread<F: FnOnce()+Send+'static>(thread: &mut ::threads::Thread, code: F) {
		// Set thread state's join handle to a thread with a pause point
		let inner_handle = thread.cpu_state.inner.clone();
		log_trace!("start_thread: {:p}", inner_handle);
		let name = "unk";
		let ptr = thread as *mut _ as usize;
		let th = std::thread::Builder::new()
			.name(name.into())
			.spawn(move || {
				// Initialise the thread-local structures
				THIS_THREAD_STATE.with(|v| {
					let mut h = v.borrow_mut();
					h.ptr = ptr as *mut _;
					h.this_state = Some(inner_handle.clone());
					});
				// Wait for the first yield
				let lh = SWITCH_LOCK.lock().unwrap();
				if ! inner_handle.running.load(Ordering::SeqCst) {
					inner_handle.condvar.wait(lh).expect("Condvar wait failed");
				}
				else {
					drop(lh);
				}
				// Run "user" code
				log_trace!("Thread started");
				(code)();
				log_trace!("Thread complete");
				// Mark the thread as being complete
				inner_handle.complete.store(true, Ordering::SeqCst);
				// Yield (which will start the next thread)
				crate::threads::yield_time();
				})
			.unwrap()
			;
		thread.cpu_state.thread_handle = Some(th);
	}
}
pub mod x86_io {
	pub unsafe fn inb(_p: u16) -> u8 { 0 }
	pub unsafe fn inw(_p: u16) -> u16 { 0 }
	pub unsafe fn inl(_p: u16) -> u32 { 0 }
	pub unsafe fn outb(_p: u16, _v: u8) { }
	pub unsafe fn outw(_p: u16, _v: u16) { }
	pub unsafe fn outl(_p: u16, _v: u32) { }
}

pub unsafe fn drop_to_user(_entry: usize, _stack: usize, _args_len: usize) -> ! {
	panic!("todo: drop_to_user");
}
pub fn puts(s: &str) {
	print!("{}", s);
}
pub fn puth(v: u64) {
	print!("{:08x}", v);
}
pub fn cur_timestamp() -> u64 {
	0
}
pub fn print_backtrace() {
}

