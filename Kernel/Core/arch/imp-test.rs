
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
				todo!("AddressSpace::pid0");
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
			panic!("TODO: Spinlock::try_lock");
		}
		pub fn lock(&self) -> HeldSpinlock<T> {
			panic!("TODO: Spinlock::lock");
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
	pub struct State;
	impl State
	{
		pub fn new(_as: &::arch::memory::virt::AddressSpace) -> State {
			todo!("threads::State::new");
		}
	}

	pub fn init_tid0_state() -> State {
		State
	}
	pub fn set_thread_ptr(_t: ::threads::ThreadPtr) {
	}
	pub fn get_thread_ptr() -> Option<::threads::ThreadPtr> {
		None
	}
	pub fn borrow_thread() -> *const ::threads::Thread {
		::core::ptr::null()
	}

	pub fn idle() {
	}
	pub fn get_idle_thread() -> ::threads::ThreadPtr {
		todo!("get_idle_thread");
	}
	pub fn switch_to(_t: ::threads::ThreadPtr) {
	}

	pub fn start_thread<F: FnOnce()+Send+'static>(_thread: &mut ::threads::Thread, _code: F) {
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
pub fn puts(_s: &str) {
}
pub fn puth(_v: u64) {
}
pub fn cur_timestamp() -> u64 {
	0
}
pub fn print_backtrace() {
}

