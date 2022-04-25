// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/time.rs
//! Kernel timing and timers
#[cfg(target_has_atomic="64")]
use ::core::sync::atomic::{Ordering,AtomicU64};

/// Timer ticks (ms)
pub type TickCount = u64;

/// Obtain the number of timer ticks since an arbitary point (system startup)
pub fn ticks() -> u64
{
	crate::arch::time::cur_timestamp()
}

/// Function called by the arch code when the main system timer ticks
pub(super) fn time_tick()
{
	super::futures::time_tick();
	//super::user_async::time_tick();
	//super::threads::time_tick();
}

/// Requests that an interrupt be raised around this target time (could be earlier or later)
/// 
/// - Earlier if there's already an earlier interrupt requested
/// - Later if the system timer rate doesn't allow that exact point
pub fn request_interrupt(ticks: TickCount)
{
	crate::arch::time::request_tick(ticks);
}

/// Records the current time on construction, and prints the elapsed time with {:?} / {}
pub struct ElapsedLogger(TickCount);
impl ElapsedLogger
{
	pub fn new() -> Self {
		ElapsedLogger(ticks())
	}
	pub fn elapsed_ticks(&self) -> TickCount {
		ticks() - self.0
	}
}
impl_fmt! {
	Debug(self,f) for ElapsedLogger {
		::core::fmt::Display::fmt(&self.elapsed_ticks(), f)
	}
	//Display(self,f) for ElapsedLogger {
	//	::core::fmt::Display::fmt(&(ticks() - self.0), f)
	//}
}

pub struct Timer
{
	expiry_time: TickCount,
}
impl Default for Timer {
	fn default() -> Self {
		Self::new()
	}
}
impl Timer
{
	pub const fn new() -> Self {
		Timer {
			expiry_time: !0,
		}
	}
	/// Returns `None` if the timer is expired, and `Some(tickcount)` if it's still to fire
	pub fn get_expiry(&self) -> Option<TickCount> {
		(self.expiry_time < ticks()).then(|| self.expiry_time)
	}
	pub fn is_expired(&self) -> bool {
		self.expiry_time < ticks()
	}
	/// Reset the timer to the given duration
	pub fn reset(&mut self, duration: u64) -> bool {
		let rv = self.expiry_time < ticks() || self.expiry_time == !0;
		self.expiry_time = ticks() + duration;
		rv
	}
	pub fn clear(&mut self) -> bool {
		let rv = self.expiry_time < ticks();
		self.expiry_time = !0;
		rv
	}
	//pub fn wait(&self) -> impl ::core::future::Future<Output=()> {
	//	
	//}
}

// TODO: Use AtomicU64 if availble, otherwise use a spinlock protected u32 pair
/// A timer used to track the last access time of a field
pub struct CacheTimer(
	#[cfg(target_has_atomic="64")]
	AtomicU64,
	#[cfg(not(target_has_atomic="64"))]
	crate::sync::Spinlock<u64>,
	);
impl Default for CacheTimer {
	fn default() -> Self {
		CacheTimer::new()
	}
}
#[cfg(target_has_atomic="64")]
impl CacheTimer
{
	pub fn new() -> Self {
		CacheTimer( AtomicU64::new(ticks()) )
	}

	pub fn bump(&self) {
		self.0.store(ticks(), Ordering::SeqCst)
	}
}
#[cfg(not(target_has_atomic="64"))]
impl CacheTimer
{
	pub fn new() -> Self {
		CacheTimer( crate::sync::Spinlock::new(ticks()) )
	}

	pub fn bump(&self) {
		*self.0.lock() = ticks();
	}
}

// vim: ft=rust

