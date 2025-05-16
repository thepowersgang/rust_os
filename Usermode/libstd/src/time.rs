//! Time handling

/// Duration (difference in times)
#[derive(Debug,Copy,Clone)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Duration(u64);

/// A monotonic instant
// Encoded as microseconds since system startup
#[derive(Debug,Copy,Clone)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Instant(u64);
impl Instant
{
	pub fn now() -> Instant {
		Instant(::syscalls::system_ticks())
	}
	pub fn duration_since(&self, earlier: &Self) -> Duration {
		assert!(earlier.0 <= self.0);
		Duration(self.0 - earlier.0)
	}
	pub fn checked_duration_since(&self, earlier: &Self) -> Option<Duration> {
		if earlier.0 <= self.0 {
			Some(Duration(self.0 - earlier.0))
		}
		else {
			None
		}
	}
}

/*
/// System time
// Encoded as microseconds since the unix epoch UTC (with 64 bits, this should last about 60 thousand years)
pub struct SystemTime(i64);
impl SystemTime
{
	pub fn now() -> Self {
		Instant(::syscalls::wall_clock())
	}
}

/// The unix epoch
pub const UNIX_EPOCH: SystemTime = SystemTime(0);
*/