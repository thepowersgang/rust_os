
#![macro_escape]	// Let macros be accessible by parent

macro_rules! log( ($lvl:expr, $modname:expr, $($arg:tt)*) => (
	if ::logging::enabled($lvl, $modname)
	{
		use core::fmt::FormatWriter;
		let _ = write!(&mut ::logging::getstream($lvl, $modname), $($arg)*);
	}
	))
macro_rules! log_panic(   ($($arg:tt)*) => (log!(::logging::LevelPanic, module_path!(), $($arg)*)) )
macro_rules! log_error(   ($($arg:tt)*) => (log!(::logging::LevelError, module_path!(), $($arg)*)) )
macro_rules! log_warning( ($($arg:tt)*) => (log!(::logging::LevelWarning, module_path!(), $($arg)*)) )
macro_rules! log_notice(  ($($arg:tt)*) => (log!(::logging::LevelNotice, module_path!(), $($arg)*)) )
macro_rules! log_log(     ($($arg:tt)*) => (log!(::logging::LevelLog, module_path!(), $($arg)*)) )
macro_rules! log_debug(   ($($arg:tt)*) => (log!(::logging::LevelDebug, module_path!(), $($arg)*)) )
macro_rules! log_trace( ($fmt:expr $(, $arg:expr)*) => (
	log!(::logging::LevelTrace, module_path!(), concat!("L{}: ",$fmt), line!() $(, $arg)*)
	) )

// vim: ft=rust

