// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/sync/atomic.rs
// - Blocking synchronisation primitives


use core::sync::atomic::Ordering;
use core::intrinsics;

pub unsafe trait ValidAtomic {}

unsafe impl ValidAtomic for usize {}
unsafe impl ValidAtomic for u32 {}
// TODO: I'm not 100% sure about this
unsafe impl ValidAtomic for u64 {}

#[repr(C)]
pub struct AtomicValue<T: Copy>(::core::cell::UnsafeCell<T>);
unsafe impl<T: Copy+Send+Sync> Sync for AtomicValue<T> {}
unsafe impl<T: Copy+Send+Sync> Send for AtomicValue<T> {}
unsafe impl<T: 'static+Copy+ValidAtomic> ::lib::POD for AtomicValue<T> {}

impl<T: Copy+Send+Sync+Default> Default for AtomicValue<T>
{
	fn default() -> Self {
		AtomicValue::new(Default::default())
	}
}
impl<T: Copy+Send+Sync> AtomicValue<T>
{
	pub const fn new(val: T) -> AtomicValue<T> {
		AtomicValue( ::core::cell::UnsafeCell::new(val) )
	}
}

impl<T: Copy+Send+Sync/*+ValidAtomic*/> AtomicValue<T>
{
	/// Unconditionally loads
	pub fn load(&self, order: Ordering) -> T {
		// SAFE: Atomic
		unsafe {
			let dst = self.0.get();
			match order {
			Ordering::Acquire => intrinsics::atomic_load_acq(dst),
			Ordering::Relaxed => intrinsics::atomic_load_relaxed(dst),
			Ordering::SeqCst  => intrinsics::atomic_load(dst),
			Ordering::Release => panic!("there is no such thing as a release load"),
			Ordering::AcqRel  => panic!("there is no such thing as an acquire/release load"),
			_ => panic!("Ordering {:?}", order),
			}
		}
	}
	/// Unconditionally stores
	pub fn store(&self, val: T, order: Ordering) {
		// SAFE: Atomic
		unsafe {
			let dst = self.0.get();
			match order {
			Ordering::Release => intrinsics::atomic_store_rel(dst, val),
			Ordering::Relaxed => intrinsics::atomic_store_relaxed(dst, val),
			Ordering::SeqCst  => intrinsics::atomic_store(dst, val),
			Ordering::Acquire => panic!("there is no such thing as an acquire store"),
			Ordering::AcqRel  => panic!("there is no such thing as an acquire/release store"),
			_ => panic!("Ordering {:?}", order),
			}
		}
	}
	/// Exchange
	pub fn swap(&self, val: T, order: Ordering) -> T {
		// SAFE: Atomic
		unsafe {
			let dst = self.0.get();
			match order {
			Ordering::Acquire => intrinsics::atomic_xchg_acq(dst, val),
			Ordering::Release => intrinsics::atomic_xchg_rel(dst, val),
			Ordering::AcqRel  => intrinsics::atomic_xchg_acqrel(dst, val),
			Ordering::Relaxed => intrinsics::atomic_xchg_relaxed(dst, val),
			Ordering::SeqCst  => intrinsics::atomic_xchg(dst, val),
			_ => panic!("Ordering {:?}", order),
			}
		}
	}

	/// Compare and exchange, returns old value and writes `new` if it was equal to `val`
	pub fn compare_and_swap(&self, old: T, new: T, order: Ordering) -> T {
		self.compare_exchange(old, new, order).0
	}
	/// Compare and exchange, returns old value and writes `new` if it was equal to `val`
	pub fn compare_exchange(&self, old: T, new: T, order: Ordering) -> (T, bool) {
		// SAFE: Atomic
		unsafe {
			let dst = self.0.get();
			match order {
			Ordering::Acquire => intrinsics::atomic_cxchg_acq(dst, old, new),
			Ordering::Release => intrinsics::atomic_cxchg_rel(dst, old, new),
			Ordering::AcqRel  => intrinsics::atomic_cxchg_acqrel(dst, old, new),
			Ordering::Relaxed => intrinsics::atomic_cxchg_relaxed(dst, old, new),
			Ordering::SeqCst  => intrinsics::atomic_cxchg(dst, old, new),
			_ => panic!("Ordering {:?}", order),
			}
		}
	}
}

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
			_ => panic!("Ordering {:?}", order),
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
			_ => panic!("Ordering {:?}", order),
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
			Ordering::SeqCst  => intrinsics::atomic_xchg(dst, val),
			_ => panic!("Ordering {:?}", order),
			}
		}
	}
	/// Compare and exchange, returns old value and writes `new` if it was equal to `val`
	pub fn compare_and_swap(&self, old: u32, new: u32, order: Ordering) -> u32 {
		self.compare_exchange(old, new, order).0
	}
	/// Compare and exchange, returns old value and writes `new` if it was equal to `val`
	pub fn compare_exchange(&self, old: u32, new: u32, order: Ordering) -> (u32, bool) {
		// SAFE: Atomic
		unsafe {
			let dst = self.0.get();
			match order {
			Ordering::Acquire => intrinsics::atomic_cxchg_acq(dst, old, new),
			Ordering::Release => intrinsics::atomic_cxchg_rel(dst, old, new),
			Ordering::AcqRel  => intrinsics::atomic_cxchg_acqrel(dst, old, new),
			Ordering::Relaxed => intrinsics::atomic_cxchg_relaxed(dst, old, new),
			Ordering::SeqCst  => intrinsics::atomic_cxchg(dst, old, new),
			_ => panic!("Ordering {:?}", order),
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
			Ordering::SeqCst  => intrinsics::atomic_xadd(dst, val),
			_ => panic!("Ordering {:?}", order),
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
			Ordering::SeqCst  => intrinsics::atomic_xsub(dst, val),
			_ => panic!("Ordering {:?}", order),
			}
		}
	}

	// TODO: AND, OR, XOR
}
