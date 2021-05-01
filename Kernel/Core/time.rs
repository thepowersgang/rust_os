// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/time.rs
//! Kernel timing and timers
//use ::core::sync::atomic::{Ordering,AtomicU64};

/// Timer ticks (ms)
pub type TickCount = u64;

/// Obtain the number of timer ticks since an arbitary point (system startup)
pub fn ticks() -> u64
{
	::arch::cur_timestamp()
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


// TODO: Use AtomicU64 if availble, otherwise use a spinlock protected u32 pair
pub struct CacheTimer(::sync::atomic::AtomicValue<TickCount>);
impl Default for CacheTimer {
	fn default() -> Self {
		CacheTimer::new()
	}
}
impl CacheTimer
{
	pub fn new() -> Self {
		CacheTimer( ::sync::atomic::AtomicValue::new(ticks()) )
	}

	pub fn bump(&self) {
		self.0.store(ticks(), ::core::sync::atomic::Ordering::SeqCst)
	}
}

// vim: ft=rust

