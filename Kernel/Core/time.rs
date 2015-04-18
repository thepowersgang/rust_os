//
//
//

/// Timer ticks (ms)
pub type TickCount = u64;

/// Obtain the number of timer ticks since an arbitary point (system startup)
pub fn ticks() -> u64
{
	::arch::cur_timestamp()
}

// vim: ft=rust

