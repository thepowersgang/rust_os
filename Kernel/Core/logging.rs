// Module: ::logging
//
//
#![macro_escape]	// Let macros be accessible by parent

use core::fmt::FormatWriter;

pub enum Level
{
	LevelPanic,	// Everything broke
	LevelError,	// Something broke
	LevelWarning,	// Recoverable
	LevelNotice,	// Odd
	LevelInfo,	// Interesting
	LevelLog,	// General
	LevelDebug,	// What
	LevelTrace,	// Where
}

struct LoggingFormatter;
//{
//	lock_handle: ::kstd::sync::MutexHandle,
//}

impl ::core::fmt::Char for Level
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		write!(f, "{:c}",
			match *self
			{
			LevelPanic   => 'k',
			LevelError   => 'e',
			LevelWarning => 'w',
			LevelNotice  => 'n',
			LevelInfo    => 'i',
			LevelLog     => 'l',
			LevelDebug   => 'd',
			LevelTrace   => 't',
			}
			)
	}
}

impl LoggingFormatter
{
	pub fn new() -> LoggingFormatter
	{
		LoggingFormatter	// {}
	}
}

impl ::core::fmt::FormatWriter for LoggingFormatter
{
	fn write(&mut self, bytes: &[u8]) -> ::core::fmt::Result
	{
		::arch::puts(::core::str::from_utf8(bytes).unwrap());
		::core::result::Ok( () )
	}
}
impl ::core::ops::Drop for LoggingFormatter
{
	fn drop(&mut self)
	{
		::arch::puts("\n");
	}
}

pub fn enabled(level: Level, modname: &str) -> bool
{
	true
}

pub fn getstream(level: Level, modname: &str) -> LoggingFormatter
{
	assert!( enabled(level, modname) );
	let mut rv = LoggingFormatter::new();
	let _ = write!(&mut rv, "{:8u}{:c} [{:6s}] - ", ::time::ticks(), level, modname);
	rv
}


macro_rules! log( ($lvl:expr, $modname:expr, $($arg:tt)*) => (
	if ::logging::enabled($lvl, $modname)
	{
		use core::fmt::FormatWriter;
		let _ = write!(&mut ::logging::getstream($lvl, $modname), $($arg)*);
	}
	))
macro_rules! log_notice( ($($arg:tt)*) => (log!(::logging::LevelLog, "TODO", $($arg)*)) )
macro_rules! log_log( ($($arg:tt)*) => (log!(::logging::LevelNotice, "TODO", $($arg)*)) )
macro_rules! log_debug( ($($arg:tt)*) => (log!(::logging::LevelLog, "TODO", $($arg)*)) )

// vim: ft=rust

