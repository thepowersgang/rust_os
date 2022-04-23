pub mod memory {
	pub const PAGE_SIZE: usize = 0x1000;
	pub type PAddr = u64;
	pub type VAddr = usize;

	pub mod addresses {
		pub const USER_END:       usize = 0x00008000_00000000;
		
		/// Start of the kernel heap
		pub const HEAP_START:     usize = 0xFFFF8000_00000000;
		/// End of the kernel heap
		pub const HEAP_END:       usize = 0xFFFF9000_00000000;
		
		/// Start of the kernel module load area
		pub const MODULES_BASE:   usize = HEAP_END;
		/// End of the kernel module load area
		pub const MODULES_END:    usize = 0xFFFFA000_00000000;
		
		/// Start of the stacks region
		pub const STACKS_BASE:    usize = MODULES_END;
		/// End of the stacks region
		pub const STACKS_END:     usize = 0xFFFFB000_00000000;
		
		/// Start of the hardware mapping region
		pub const HARDWARE_BASE:  usize = STACKS_END;
		/// End of the hardware mapping region
		pub const HARDWARE_END:   usize = 0xFFFF_C000_00000000;
//
//		// Physical memory reference counting base:
//		//  - D-C = 1<<(32+12) = (1 << 44)
//		//  - / 4 = (1 << 42) frames, = 4 trillion = 16PB RAM
//		pub const PMEMREF_BASE:   usize = HARDWARE_END;
//		pub const PMEMREF_END:    usize = 0xFFFF_D000_00000000;
//		const MAX_FRAME_IDX: usize = (PMEMREF_END - PMEMREF_BASE) / 4;	// 32-bit integer each
//		pub const PMEMBM_BASE:	  usize = PMEMREF_END;
//		pub const PMEMBM_END:     usize = PMEMBM_BASE + MAX_FRAME_IDX / 8;	// 8 bits per byte in bitmap
//		
		pub const BUMP_START:	usize = 0xFFFF_E000_00000000;
		pub const BUMP_END  :	usize = 0xFFFF_F000_00000000;
//		// Most of F is free
//		
		pub const STACK_SIZE: usize = 0x4000;
//		
//		#[doc(hidden)]
//		/// Start of the fractal mapping
//		pub const FRACTAL_BASE:    usize = 0xFFFFFE00_00000000;	// PML4[508]
//		#[doc(hidden)]
//		pub const IDENT_START:    usize = 0xFFFFFFFF_80000000;	// PML4[511] (plus some)
//		#[doc(hidden)]
//		pub const IDENT_END:      usize = IDENT_START + 0x400000;	// 4MiB
//		
//		/// 
//		pub const TEMP_BASE: usize = IDENT_END;
//		pub const TEMP_END:  usize = 0xFFFFFFFF_FFFF0000;	// Leave the last 16 pages free
		pub fn is_global(addr: usize) -> bool
		{
			if addr < USER_END {
				false
			}
			//else if addr < HEAP_START {
			//	panic!("Calling is_global on non-canonical address {:#x}", addr)
			//}
			// TODO: Kernel-side per-process data
			else {
				true
			}
		}
	}

	pub mod virt {
		pub struct AddressSpace;
		impl AddressSpace
		{
			pub fn pid0() -> AddressSpace {
				AddressSpace
			}
			pub fn new(_cstart: usize, _cend: usize) -> Result<AddressSpace,()> {
				return Ok(AddressSpace);
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
			true	// NOTE: Assume all memory is valid
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
	pub struct SpinlockInner(());
	impl SpinlockInner
	{
		pub const fn new() -> SpinlockInner {
			SpinlockInner( () )
		}
		pub fn inner_lock(&self) {
		}
		pub fn try_inner_lock_cpu(&self) -> bool {
			false
		}
		pub fn inner_release(&self) {
		}
	}


	pub struct HeldInterrupts;
	pub fn hold_interrupts() -> HeldInterrupts {
		HeldInterrupts
	}

	pub unsafe fn start_interrupts() {
	}
	pub unsafe fn stop_interrupts() {
	}
}
pub mod interrupts {
	#[derive(Default)]
	pub struct IRQHandle;
	#[derive(Debug)]
	pub struct BindError;

	pub fn bind_gsi(gsi: usize, handler: fn(*const ()), info: *const ()) -> Result<IRQHandle, BindError>
	{
		Err(BindError)
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

pub mod threads {
	pub struct State;
	impl State
	{
		pub fn new(a: &super::memory::virt::AddressSpace) -> State {
			todo!("");
		}
	}
	pub fn init_tid0_state() -> State {
		State
	}
	pub fn start_thread<F: FnOnce()+Send+'static>(thread: &crate::threads::Thread, code: F)
	{
	}

	pub fn idle() {
		// SAFE: Does nothing!
		unsafe { asm!("") }
	}
	pub fn switch_to(t: ::threads::ThreadPtr) {
	}

	pub fn get_idle_thread() -> crate::threads::ThreadPtr {
		todo!("");
	}

	pub fn set_thread_ptr(t: ::threads::ThreadPtr) {
	}
	pub fn get_thread_ptr() -> Option<::threads::ThreadPtr> {
		todo!("");
	}
	pub fn borrow_thread() -> *const ::threads::Thread {
		todo!("");
	}
}

pub mod x86_io {
	pub unsafe fn inb(_p: u16) -> u8 { panic!("calling inb on non-x86") }
	pub unsafe fn inw(_p: u16) -> u16 { panic!("calling inw on non-x86") }
	pub unsafe fn inl(_p: u16) -> u32 { panic!("calling inl on non-x86") }
	pub unsafe fn outb(_p: u16, _v: u8) {}
	pub unsafe fn outw(_p: u16, _v: u16) {}
	pub unsafe fn outl(_p: u16, _v: u32) {}
}

pub mod time {
	pub fn cur_timestamp() -> u64 {
		0
	}
	pub fn request_tick(_target_time: u64) {
		todo!("")
	}
}

pub fn puts(s: &str) {
}
pub fn puth(v: u64) {
}
pub fn print_backtrace() {
}

pub fn drop_to_user(entry: usize, stack: usize, args_len: usize) -> ! {
	loop {}
}
