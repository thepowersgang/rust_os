//
//
//
//! Achitecture-specific code

cfg_if::cfg_if!{
	if #[cfg(feature="test")] {
		#[path="imp-test.rs"]
		pub mod test;

		pub use self::test as imp;
	}
	else {
		// It would be nice to have all architectures built when running
		// in the IDE, but there's conflicts getting access to the thread
		// state.
		// Also, inline assembly.
		#[cfg(any(/* in_ide, */target_arch="x86_64" ))] pub mod amd64;
		#[cfg(any(/* in_ide, */target_arch="arm"    ))] pub mod armv7;
		#[cfg(any(/* in_ide, */target_arch="aarch64"))] pub mod armv8;
		#[cfg(any(/* in_ide, */target_arch="riscv64"))] pub mod riscv64;

		#[cfg(target_arch="x86_64")]
		pub use self::amd64 as imp;
		#[cfg(target_arch="arm")]
		pub use self::armv7 as imp;
		#[cfg(target_arch="aarch64")]
		pub use self::armv8 as imp;
		#[cfg(target_arch="riscv64")]
		pub use self::riscv64 as imp;
	}
}

// If on x86/amd64, import ACPI
#[cfg(not(feature="test"))]
#[cfg(any(arch="amd64", target_arch="x86_64"))]
pub use self::imp::acpi;


/// Memory management
pub mod memory {
	/// Physical address type
	pub type PAddr = crate::arch::imp::memory::PAddr;
	pub type VAddr = crate::arch::imp::memory::VAddr;

	/// Size of a page/frame in bytes (always a power of two)
	pub const PAGE_SIZE: usize = crate::arch::imp::memory::PAGE_SIZE;
	/// Offset mask for a page
	pub const PAGE_MASK: usize = PAGE_SIZE-1;

	/// Address space layout
	pub mod addresses {
		use crate::arch::imp::memory::addresses as imp;

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

		pub const PMEMREF_BASE: usize = imp::PMEMREF_BASE;
		pub const PMEMREF_END : usize = imp::PMEMREF_END;
		pub const PMEMBM_BASE: usize = imp::PMEMBM_BASE;
		pub const PMEMBM_END : usize = imp::PMEMBM_END;
	}
	/// Virtual memory manipulation
	pub mod virt {
		use crate::arch::imp::memory::virt as imp;
		
		/// Handle to an address space
		#[derive(Debug)]
		pub struct AddressSpace(imp::AddressSpace);
		impl AddressSpace
		{
			pub fn new(clone_start: usize, clone_end: usize) -> Result<AddressSpace,crate::memory::virt::MapError> {
				imp::AddressSpace::new(clone_start, clone_end).map(AddressSpace)
			}
			pub fn pid0() -> AddressSpace {
				AddressSpace(imp::AddressSpace::pid0())
			}
			pub fn inner(&self) -> &imp::AddressSpace {
				&self.0
			}
		}

		/// A handle to a temproarily mapped frame containing instances of 'T'
		// TODO: TempHandle doens't own the mapped frame - It probably should
		pub struct TempHandle<T>(*mut T);
		impl<T> TempHandle<T>
		{
			/// UNSAFE: User must ensure that address is valid, and that no aliasing occurs
			pub unsafe fn new(phys: crate::arch::memory::PAddr) -> TempHandle<T> {
				TempHandle( imp::temp_map(phys) )
			}
			/// Cast to another type
			pub fn into<U>(self) -> TempHandle<U> {
				let rv = TempHandle( self.0 as *mut U );
				::core::mem::forget(self);
				rv
			}
			pub fn phys_addr(&self) -> crate::memory::PAddr {
				get_phys(self.0)
			}
		}
		impl<T: crate::lib::POD> ::core::ops::Deref for TempHandle<T> {
			type Target = [T];
			fn deref(&self) -> &[T] {
				// SAFE: We should have unique access, and data is POD
				unsafe { ::core::slice::from_raw_parts(self.0, crate::PAGE_SIZE / ::core::mem::size_of::<T>()) }
			}
		}
		impl<T: crate::lib::POD> ::core::ops::DerefMut for TempHandle<T> {
			fn deref_mut(&mut self) -> &mut [T] {
				// SAFE: We should have unique access, and data is POD
				unsafe { ::core::slice::from_raw_parts_mut(self.0, crate::PAGE_SIZE / ::core::mem::size_of::<T>()) }
			}
		}
		impl<T> ::core::ops::Drop for TempHandle<T> {
			fn drop(&mut self) {
				// SAFE: Address came from a call to temp_map
				unsafe {
					imp::temp_unmap(self.0);
				}
			}
		}

		pub fn post_init() {
			imp::post_init()
		}

		#[inline]
		pub fn get_phys<T: ?Sized>(p: *const T) -> crate::memory::PAddr {
			imp::get_phys(p as *const ())
		}
		#[inline]
		pub fn is_reserved<T>(p: *const T) -> bool {
			imp::is_reserved(p)
		}
		#[inline]
		pub fn get_info<T>(p: *const T) -> Option<(crate::memory::PAddr,crate::memory::virt::ProtectionMode)> {
			imp::get_info(p)
		}

		#[inline]
		pub fn is_fixed_alloc(addr: *const (), size: usize) -> bool {
			imp::is_fixed_alloc(addr, size)
		}
		#[inline]
		pub unsafe fn fixed_alloc(p: crate::memory::PAddr, count: usize) -> Option<*mut ()> {
			imp::fixed_alloc(p, count)
		}

		#[inline]
		/// Returns `true` if the provided address can have `map` called on without needing memory allocation
		// Used in physical memory allocation to avoid recursion
		pub fn can_map_without_alloc(a: *mut ()) -> bool {
			imp::can_map_without_alloc(a)
		}

		#[inline]
		pub unsafe fn map(a: *mut (), p: crate::memory::PAddr, mode: crate::memory::virt::ProtectionMode) {
			imp::map(a, p, mode)
		}
		#[inline]
		pub unsafe fn reprotect(a: *mut (), mode: crate::memory::virt::ProtectionMode) {
			imp::reprotect(a, mode)
		}
		#[inline]
		pub unsafe fn unmap(a: *mut ()) -> Option<crate::memory::PAddr> {
			imp::unmap(a)
		}
	}
}

/// Syncronisation types (spinlock and interrupt holding)
pub mod sync {
	use super::imp::sync as imp;

	/// Lightweight protecting spinlock
	pub struct Spinlock<T>
	{
		#[doc(hidden)]
		/*pub*/ lock: imp::SpinlockInner,
		#[doc(hidden)]
		pub value: ::core::cell::UnsafeCell<T>,
	}
	unsafe impl<T: Send> Sync for Spinlock<T> {}


	impl<T> Spinlock<T>
	{
		/// Create a new spinning lock
		pub const fn new(val: T) -> Spinlock<T> {
			Spinlock {
				lock: imp::SpinlockInner::new(),
				value: ::core::cell::UnsafeCell::new(val),
			}
		}
		pub fn get_mut(&mut self) -> &mut T {
			// SAFE: &mut to lock
			unsafe { &mut *self.value.get() }
		}
		
		/// Lock this spinning lock
		//#[not_safe(irq)]
		pub fn lock(&self) -> HeldSpinlock<T>
		{
			self.lock.inner_lock();
			HeldSpinlock { lock: self }
		}

		/// Lock this spinning lock (accepting risk of panick/deadlock from IRQs)
		//#[is_safe(irq)]
		pub fn lock_irqsafe(&self) -> HeldSpinlock<T> {
			self.lock.inner_lock();
			HeldSpinlock { lock: self }
		}
		/// Attempt to acquire the lock, returning None if it is already held by this CPU
		//#[is_safe(irq)]
		pub fn try_lock_cpu(&self) -> Option<HeldSpinlock<T>>
		{
			if self.lock.try_inner_lock_cpu()
			{
				Some( HeldSpinlock { lock: self } )
			}
			else
			{
				None
			}
		}
	}
	// Some special functions on non-wrapping spinlocks
	impl Spinlock<()>
	{
		pub unsafe fn unguarded_lock(&self) {
			self.lock.inner_lock()
		}
		pub unsafe fn unguarded_release(&self) {
			self.lock.inner_release()
		}
	}
	impl<T: Default> Default for Spinlock<T>
	{
		fn default() -> Self {
			Spinlock::new(Default::default())
		}
	}

	pub struct HeldSpinlock<'lock, T: 'lock>
	{
		lock: &'lock Spinlock<T>,
	}
	impl<'lock,T> ::core::ops::Drop for HeldSpinlock<'lock, T>
	{
		fn drop(&mut self)
		{
			// SAFE: This is the RAII handle for the lock
			unsafe {
				self.lock.lock.inner_release();
			}
		}
	}

	impl<'lock,T> ::core::ops::Deref for HeldSpinlock<'lock, T>
	{
		type Target = T;
		fn deref(&self) -> &T {
			// SAFE: & to handle makes & to value valid
			unsafe { &*self.lock.value.get() }
		}
	}
	impl<'lock,T> ::core::ops::DerefMut for HeldSpinlock<'lock, T>
	{
		fn deref_mut(&mut self) -> &mut T {
			// SAFE: &mut to handle makes &mut to value valid
			unsafe { &mut *self.lock.value.get() }
		}
	}

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

	/// Architecture-specific IRQ binding error type
	pub type BindError = imp::BindError;

	/// IRQ handle (unbinds the IRQ when dropped)
	#[derive(Default)]
	pub struct IRQHandle(imp::IRQHandle);
	impl IRQHandle {
		///// Disable/mask-off this IRQ (for when processing is scheduled)
		//pub fn disable(&self) {
		//}
		///// Enable/mask-on this IRQ (for when processing is complete)
		//pub fn enable(&self) {
		//}
	}
	
	#[inline]
	/// Attach a callback to an IRQ/interrupt
	pub fn bind_gsi(gsi: usize, handler: fn(*const()), info: *const ()) -> Result<IRQHandle, BindError> {
		imp::bind_gsi(gsi, handler, info).map(|v| IRQHandle(v))
	}
}
pub mod boot {
	use super::imp::boot as imp;

	#[inline]
	pub fn get_boot_string() -> &'static str {
		imp::get_boot_string()
	}
	#[inline]
	pub fn get_video_mode() -> Option<crate::metadevs::video::bootvideo::VideoMode> {
		imp::get_video_mode()
	}
	#[inline]
	pub fn get_memory_map() -> &'static [crate::memory::MemoryMapEnt] {
		imp::get_memory_map()
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
	/// Called once SMP is "safe" to start
	pub fn init_smp() {
		imp::init_smp()
	}

	#[inline]
	pub fn set_thread_ptr(t: crate::threads::ThreadPtr) {
		imp::set_thread_ptr(t)
	}
	#[inline]
	pub fn get_thread_ptr() -> Option<crate::threads::ThreadPtr> {
		imp::get_thread_ptr()
	}
	#[inline]
	pub fn borrow_thread() -> *const crate::threads::Thread {
		imp::borrow_thread()
	}

	#[inline]
	pub fn idle(held_interrupts: super::sync::HeldInterrupts) {
		imp::idle(held_interrupts)
	}
	#[inline]
	pub fn get_idle_thread() -> crate::threads::ThreadPtr {
		imp::get_idle_thread()
	}
	#[inline]
	pub fn switch_to(t: crate::threads::ThreadPtr) {
		imp::switch_to(t)
	}

	#[inline]
	pub fn start_thread<F: FnOnce()+Send+'static>(thread: &mut crate::threads::Thread, code: F) {
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

pub mod time {
	use crate::arch::imp::time as imp;

	#[inline]
	pub fn request_tick(target_time: u64) {
		imp::request_tick(target_time)
	}

	#[inline]
	pub fn cur_timestamp() -> u64 {
		imp::cur_timestamp()
	}
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
pub fn print_backtrace() {
	imp::print_backtrace()
}

#[inline]
pub unsafe fn drop_to_user(entry: usize, stack: usize, args_len: usize) -> ! {
	imp::drop_to_user(entry, stack, args_len)
}

