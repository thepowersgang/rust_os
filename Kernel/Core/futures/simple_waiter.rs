//!
//! 
use core::sync::atomic::{AtomicUsize,Ordering};
use core::task;
use crate::sync::EventChannel;

pub struct SimpleWaiter
{
	inner: SimpleWaiterRef,
}

impl SimpleWaiter
{
	pub fn new() -> SimpleWaiter {
		SimpleWaiter {
			inner: if let Some(v) = PooledHandle::acquire() {
					SimpleWaiterRef::Pooled(v)
				}
				else {
					SimpleWaiterRef::Owned(::alloc::sync::Arc::new(Inner::new()))
				},
			}
	}

	pub fn sleep(&self) {
		self.inner.sleep();
	}

	pub fn raw_waker(&self) -> task::RawWaker {
		self.inner.raw_waker()
	}
}

/// Actual inner data
struct Inner
{
	ref_count: AtomicUsize,
	ec: EventChannel,
}
impl Inner
{
	const fn new() -> Inner {
		Inner {
			ref_count: AtomicUsize::new(0),
			ec: EventChannel::new(),
		}
	}

    fn sleep(&self) {
        //log_trace!("sleep({:p})", self);
        self.ec.sleep();
    }
	
	fn rw_wake_by_ref(raw_self: *const ()) {
        //log_trace!("wake({:p})", raw_self);
		let v = unsafe { &*(raw_self as *const Self) };
		v.ec.post();
	}
}

// TODO: Make a pool of waiters, so they can outlive the stack frame.
static WAITER_LOCK: crate::sync::Spinlock<()> = crate::sync::Spinlock::new( () );
static WAITER_POOL: [Inner; 8] = [
	Inner::new(), Inner::new(), Inner::new(), Inner::new(),
	Inner::new(), Inner::new(), Inner::new(), Inner::new(),
	];
struct PooledHandle(&'static Inner);
impl PooledHandle {
	fn acquire() -> Option<Self> {
		let _lh = WAITER_LOCK.lock();
		for v in WAITER_POOL.iter() {
			if v.ref_count.compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst) == Ok(0) {
				return Some(PooledHandle(v));
			}
		}
		None
	}
}
impl ::core::clone::Clone for PooledHandle {
	fn clone(&self) -> Self {
		self.0.ref_count.fetch_add(1, Ordering::SeqCst);
		Self(self.0)
	}
}
impl ::core::ops::Drop for PooledHandle {
	fn drop(&mut self) {
		self.0.ref_count.fetch_sub(1, Ordering::SeqCst);
	}
}

enum SimpleWaiterRef {
	Owned(::alloc::sync::Arc<Inner>),
	Pooled(PooledHandle),
}
impl SimpleWaiterRef {
	fn sleep(&self) {
		match self
        {
        SimpleWaiterRef::Owned(v) => v.sleep(),
        SimpleWaiterRef::Pooled(v) => v.0.sleep(),
        }
	}
	fn raw_waker(&self) -> task::RawWaker {
		match self
		{
		SimpleWaiterRef::Owned(v) => {
			use ::alloc::sync::Arc;
			fn make(v: Arc<Inner>) -> task::RawWaker {
				static VTABLE: task::RawWakerVTable = task::RawWakerVTable::new(
					/*clone:*/ rw_clone,
					/*wake:*/ |v| { Inner::rw_wake_by_ref(v); rw_drop(v) },
					/*wake_by_ref:*/ Inner::rw_wake_by_ref,
					/*drop:*/ rw_drop,
					);
				let v = Arc::into_raw(v);
				task::RawWaker::new(v as *const (), &VTABLE)
			}
			fn rw_clone(raw_self: *const ()) -> task::RawWaker {
				unsafe {
                    let raw_self = raw_self as *const Inner;
					let r = Arc::from_raw(raw_self);
                    Arc::increment_strong_count(raw_self);  // Effectively clone
					make(r)
				}
			}
			fn rw_drop(raw_self: *const ()) {
				unsafe { Arc::from_raw(raw_self as *const Inner) };
			}
			make(v.clone())
			},
		SimpleWaiterRef::Pooled(v) => {
			fn make(v: &'static Inner) -> task::RawWaker {
				static VTABLE: task::RawWakerVTable = task::RawWakerVTable::new(
					/*clone:*/ rw_clone,
					/*wake:*/ |v| { Inner::rw_wake_by_ref(v); rw_drop(v) },
					/*wake_by_ref:*/ Inner::rw_wake_by_ref,
					/*drop:*/ rw_drop,
					);
				v.ref_count.fetch_add(1, Ordering::SeqCst);
				task::RawWaker::new(v as *const _ as *const (), &VTABLE)
			}
			fn rw_clone(raw_self: *const ()) -> task::RawWaker {
				make(unsafe { &*(raw_self as *const Inner) })
			}
			fn rw_drop(raw_self: *const ()) {
				let v = unsafe { &*(raw_self as *const Inner) };
				v.ref_count.fetch_sub(1, Ordering::SeqCst);
			}
			make(v.0)
			},
		}
	}
}