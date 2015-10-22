// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/mod.rs
// - Blocking synchronisation primitives
pub use arch::sync::Spinlock;
pub use arch::sync::hold_interrupts;

pub use sync::mutex::Mutex;
pub use sync::semaphore::Semaphore;
pub use sync::rwlock::RwLock;
pub use sync::event_channel::{EventChannel,EVENTCHANNEL_INIT};

#[macro_use]
pub mod mutex;

pub mod semaphore;

pub mod rwlock;

pub mod event_channel;



use core::sync::atomic::Ordering;
use core::intrinsics;
pub struct AtomicU32(::core::cell::UnsafeCell<u32>);
unsafe impl Sync for AtomicU32 {}
unsafe impl Send for AtomicU32 {}
impl Default for AtomicU32 {
	fn default() -> AtomicU32 {
		AtomicU32::new(0)
	}
}
impl AtomicU32 {
	pub const fn new(val: u32) -> AtomicU32 {
		AtomicU32( ::core::cell::UnsafeCell::new(val) )
	}
	/// Unconditionally loads
	pub fn load(&self, order: Ordering) -> u32 {
		// SAFE: Atomic
		unsafe {
			let dst = self.0.get();
			match order {
			Ordering::Acquire => intrinsics::atomic_load_acq(dst),
			Ordering::Relaxed => intrinsics::atomic_load_relaxed(dst),
			Ordering::SeqCst  => intrinsics::atomic_load(dst),
			Ordering::Release => panic!("there is no such thing as a release load"),
			Ordering::AcqRel  => panic!("there is no such thing as an acquire/release load"),
			}
		}
	}
	/// Unconditionally stores
	pub fn store(&self, val: u32, order: Ordering) {
		// SAFE: Atomic
		unsafe {
			let dst = self.0.get();
			match order {
			Ordering::Release => intrinsics::atomic_store_rel(dst, val),
			Ordering::Relaxed => intrinsics::atomic_store_relaxed(dst, val),
			Ordering::SeqCst  => intrinsics::atomic_store(dst, val),
			Ordering::Acquire => panic!("there is no such thing as an acquire store"),
			Ordering::AcqRel  => panic!("there is no such thing as an acquire/release store"),
			}
		}
	}
	/// Exchange
	pub fn swap(&self, val: u32, order: Ordering) -> u32 {
		// SAFE: Atomic
		unsafe {
			let dst = self.0.get();
			match order {
			Ordering::Acquire => intrinsics::atomic_xchg_acq(dst, val),
			Ordering::Release => intrinsics::atomic_xchg_rel(dst, val),
			Ordering::AcqRel  => intrinsics::atomic_xchg_acqrel(dst, val),
			Ordering::Relaxed => intrinsics::atomic_xchg_relaxed(dst, val),
			Ordering::SeqCst  => intrinsics::atomic_xchg(dst, val)
			}
		}
	}
	/// Compare and exchange, returns old value and writes `new` if it was equal to `val`
	pub fn compare_and_swap(&self, old: u32, new: u32, order: Ordering) -> u32 {
		// SAFE: Atomic
		unsafe {
			let dst = self.0.get();
			match order {
			Ordering::Acquire => intrinsics::atomic_cxchg_acq(dst, old, new),
			Ordering::Release => intrinsics::atomic_cxchg_rel(dst, old, new),
			Ordering::AcqRel  => intrinsics::atomic_cxchg_acqrel(dst, old, new),
			Ordering::Relaxed => intrinsics::atomic_cxchg_relaxed(dst, old, new),
			Ordering::SeqCst  => intrinsics::atomic_cxchg(dst, old, new),
			}
		}
	}

	pub fn fetch_add(&self, val: u32, order: Ordering) -> u32 {
		// SAFE: Atomic
		unsafe {
			let dst = self.0.get();
			match order {
			Ordering::Acquire => intrinsics::atomic_xadd_acq(dst, val),
			Ordering::Release => intrinsics::atomic_xadd_rel(dst, val),
			Ordering::AcqRel  => intrinsics::atomic_xadd_acqrel(dst, val),
			Ordering::Relaxed => intrinsics::atomic_xadd_relaxed(dst, val),
			Ordering::SeqCst  => intrinsics::atomic_xadd(dst, val)
			}
		}
	}
	pub fn fetch_sub(&self, val: u32, order: Ordering) -> u32 {
		// SAFE: Atomic
		unsafe {
			let dst = self.0.get();
			match order {
			Ordering::Acquire => intrinsics::atomic_xsub_acq(dst, val),
			Ordering::Release => intrinsics::atomic_xsub_rel(dst, val),
			Ordering::AcqRel  => intrinsics::atomic_xsub_acqrel(dst, val),
			Ordering::Relaxed => intrinsics::atomic_xsub_relaxed(dst, val),
			Ordering::SeqCst  => intrinsics::atomic_xsub(dst, val)
			}
		}
	}

	// TODO: AND, OR, XOR
}

// vim: ft=rust

