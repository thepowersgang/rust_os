//
//
//
//! Achitecture-specific code

#[macro_use]
#[cfg(arch="amd64")] #[path="amd64/mod.rs"]
#[doc(hidden)]
pub mod imp;	// Needs to be pub for exports to be avaliable

#[macro_use]
#[cfg(arch="armv7")] #[path="armv7/mod.rs"]
#[doc(hidden)]
pub mod imp;

// If on x86/amd64, import ACPI
#[cfg(arch="amd64")]
pub use self::imp::acpi;


/// Memory management
pub mod memory {
	/// Physical address type
	pub type PAddr = ::arch::imp::memory::PAddr;
	pub type VAddr = ::arch::imp::memory::VAddr;

	/// Size of a page/frame in bytes (always a power of two)
	pub const PAGE_SIZE: usize = ::arch::imp::memory::PAGE_SIZE;
	/// Offset mask for a page
	pub const PAGE_MASK: usize = (PAGE_SIZE-1);

	/// Address space layout
	pub mod addresses {
		use arch::imp::memory::addresses as imp;

		#[inline]
		/// Returns `true` if the passed address is valid in every address space
		pub fn is_global(addr: usize) -> bool {
			imp::is_global(addr)
		}

		/// Size of a single kernel statck
		pub const STACK_SIZE: usize = imp::STACK_SIZE;

		/// Last first address after the user-controlled region
		pub const USER_END: usize = imp::USER_END;

		/// Start of the kernel stack region
		pub const STACKS_BASE: usize = imp::STACKS_BASE;
		/// End of the kernel stack region
		pub const STACKS_END : usize = imp::STACKS_END ;

		/// Start of hardware mappings
		pub const HARDWARE_BASE: usize = imp::HARDWARE_BASE;
		/// End of hardware mappings
		pub const HARDWARE_END : usize = imp::HARDWARE_END ;

		/// Start of the heap reservation
		pub const HEAP_START: usize = imp::HEAP_START;
		/// End of the heap reservation
		pub const HEAP_END : usize = imp::HEAP_END ;

		pub const BUMP_START: usize = imp::BUMP_START;
		pub const BUMP_END: usize = imp::BUMP_END;
	}
	/// Virtual memory manipulation
	pub mod virt {
		use arch::imp::memory::virt as imp;
		
		/// TODO: Wrap this to ensure a consistent API
		pub type AddressSpace = imp::AddressSpace;

		/// A handle to a temproarily mapped frame containing instances of 'T'
		pub struct TempHandle<T>(*mut T);
		impl<T> TempHandle<T>
		{
			/// UNSAFE: User must ensure that address is valid, and that no aliasing occurs
			pub unsafe fn new(phys: ::arch::memory::PAddr) -> TempHandle<T> {
				TempHandle( imp::temp_map(phys) )
			}
			/// Cast to another type
			pub fn into<U>(self) -> TempHandle<U> {
				let rv = TempHandle( self.0 as *mut U );
				::core::mem::forget(self);
				rv
			}
			pub fn phys_addr(&self) -> ::memory::PAddr {
				get_phys(self.0)
			}
		}
		impl<T: ::lib::POD> ::core::ops::Deref for TempHandle<T> {
			type Target = [T];
			fn deref(&self) -> &[T] {
				// SAFE: We should have unique access, and data is POD
				unsafe { ::core::slice::from_raw_parts(self.0, ::PAGE_SIZE / ::core::mem::size_of::<T>()) }
			}
		}
		impl<T: ::lib::POD> ::core::ops::DerefMut for TempHandle<T> {
			fn deref_mut(&mut self) -> &mut [T] {
				// SAFE: We should have unique access, and data is POD
				unsafe { ::core::slice::from_raw_parts_mut(self.0, ::PAGE_SIZE / ::core::mem::size_of::<T>()) }
			}
		}
		impl<T> ::core::ops::Drop for TempHandle<T> {
			fn drop(&mut self) {
				// SAFE: Address came from a capp to temp_map
				unsafe {
					imp::temp_unmap(self.0);
				}
			}
		}

		pub fn post_init() {
			imp::post_init()
		}

		#[inline]
		pub fn get_phys<T>(p: *const T) -> ::memory::PAddr {
			imp::get_phys(p)
		}
		#[inline]
		pub fn is_reserved<T>(p: *const T) -> bool {
			imp::is_reserved(p)
		}
		#[inline]
		pub fn get_info<T>(p: *const T) -> Option<(::memory::PAddr,::memory::virt::ProtectionMode)> {
			imp::get_info(p)
		}

		#[inline]
		pub fn is_fixed_alloc(addr: *const (), size: usize) -> bool {
			imp::is_fixed_alloc(addr, size)
		}
		#[inline]
		pub unsafe fn fixed_alloc(p: ::memory::PAddr, count: usize) -> Option<*mut ()> {
			imp::fixed_alloc(p, count)
		}

		#[inline]
		/// Returns `true` if the provided address can have `map` called on without needing memory allocation
		pub fn can_map_without_alloc(a: *mut ()) -> bool {
			imp::can_map_without_alloc(a)
		}

		#[inline]
		pub unsafe fn map(a: *mut (), p: ::memory::PAddr, mode: ::memory::virt::ProtectionMode) {
			imp::map(a, p, mode)
		}
		#[inline]
		pub unsafe fn reprotect(a: *mut (), mode: ::memory::virt::ProtectionMode) {
			imp::reprotect(a, mode)
		}
		#[inline]
		pub unsafe fn unmap(a: *mut ()) -> Option<::memory::PAddr> {
			imp::unmap(a)
		}
	}
	/// Physical memory state tracking
	pub mod phys {
		use arch::imp::memory::phys as imp;

		#[inline]
		pub fn ref_frame(frame_idx: u64) {
			imp::ref_frame(frame_idx)
		}
		#[inline]
		/// Decrement the "multi-reference" count associated with a frame, returning the previous value.
		pub fn deref_frame(frame_idx: u64) -> u32 {
			imp::deref_frame(frame_idx)
		}
		#[inline]
		pub fn get_multiref_count(frame_idx: u64) -> u32 {
			imp::get_multiref_count(frame_idx)
		}

		#[inline]
		/// Returns true if the frame was marked as allocated
		pub fn mark_free(frame_idx: u64) -> bool {
			imp::mark_free(frame_idx)
		}
		#[inline]
		/// Mark a frame as "allocated"
		pub fn mark_used(frame_idx: u64) {
			imp::mark_used(frame_idx)
		}
	}
}

/// Syncronisation types (spinlock and interrupt holding)
pub mod sync {
	use super::imp::sync as imp;

	/// Busy-waiting spin-lock type
	pub type Spinlock<T> = imp::Spinlock<T>;
	pub type HeldSpinlock<'a, T: 'a> = imp::HeldSpinlock<'a, T>;
	pub type HeldInterrupts = imp::HeldInterrupts;

	#[inline]
	/// Halt interrupts and return a RAII handle
	pub fn hold_interrupts() -> HeldInterrupts {
		imp::hold_interrupts()
	}


	/// UNSAFE: Not strictly speaking...
	pub unsafe fn stop_interrupts() {
		imp::stop_interrupts()
	}
	/// UNSAFE: Can be used to break `hold_interrupts`, and other related assumptions
	pub unsafe fn start_interrupts() {
		imp::start_interrupts()
	}
}
pub mod interrupts {
	use super::imp::interrupts as imp;

	pub type BindError = imp::BindError;
	pub type IRQHandle = imp::IRQHandle;

	
	#[inline]
	pub fn bind_gsi(gsi: usize, handler: fn(*const()), info: *const ()) -> Result<IRQHandle, BindError> {
		imp::bind_gsi(gsi, handler, info)
	}
}
pub mod boot {
	use super::imp::boot as imp;

	#[inline]
	pub fn get_boot_string() -> &'static str {
		imp::get_boot_string()
	}
	#[inline]
	pub fn get_video_mode() -> Option<::metadevs::video::bootvideo::VideoMode> {
		imp::get_video_mode()
	}
	#[inline]
	pub fn get_memory_map() -> &'static [::memory::MemoryMapEnt] {
		imp::get_memory_map()
	}
}
pub mod pci {
	use super::imp::pci as imp;

	#[inline]
	pub fn read(a: u32) -> u32 {
		imp::read(a)
	}
	#[inline]
	pub fn write(a: u32, v: u32) {
		imp::write(a, v)
	}
}
pub mod threads {
	use super::imp::threads as imp;

	pub type State = imp::State;

	#[inline]
	pub fn init_tid0_state() -> State {
		imp::init_tid0_state()
	}

	#[inline]
	pub fn set_thread_ptr(t: ::threads::ThreadPtr) {
		imp::set_thread_ptr(t)
	}
	#[inline]
	pub fn get_thread_ptr() -> Option<::threads::ThreadPtr> {
		imp::get_thread_ptr()
	}
	#[inline]
	pub fn borrow_thread() -> *const ::threads::Thread {
		imp::borrow_thread()
	}

	#[inline]
	pub fn idle() {
		imp::idle()
	}
	#[inline]
	pub fn get_idle_thread() -> ::threads::ThreadPtr {
		imp::get_idle_thread()
	}
	#[inline]
	pub fn switch_to(t: ::threads::ThreadPtr) {
		imp::switch_to(t)
	}

	#[inline]
	pub fn start_thread<F: FnOnce()+Send+'static>(thread: &mut ::threads::Thread, code: F) {
		imp::start_thread(thread, code)
	}
}

/// x86 IO bus accesses
pub mod x86_io {
	use super::imp::x86_io as imp;

	#[inline]
	pub unsafe fn inb(p: u16) -> u8 { imp::inb(p) }
	#[inline]
	pub unsafe fn inw(p: u16) -> u16 { imp::inw(p) }
	#[inline]
	pub unsafe fn inl(p: u16) -> u32 { imp::inl(p) }
	#[inline]
	pub unsafe fn outb(p: u16, v: u8) { imp::outb(p, v) }
	#[inline]
	pub unsafe fn outw(p: u16, v: u16) { imp::outw(p, v) }
	#[inline]
	pub unsafe fn outl(p: u16, v: u32) { imp::outl(p, v) }
}


#[inline]
pub fn puts(s: &str) {
	imp::puts(s);
}
#[inline]
pub fn puth(v: u64) {
	imp::puth(v)
}

#[inline]
pub fn cur_timestamp() -> u64 {
	imp::cur_timestamp()
}
#[inline]
pub fn print_backtrace() {
	imp::print_backtrace()
}

#[inline]
pub unsafe fn drop_to_user(entry: usize, stack: usize, args_len: usize) -> ! {
	imp::drop_to_user(entry, stack, args_len)
}

