
#[inline(never)]
#[doc(hidden)]
#[cfg_attr(not(test_shim),req_safe(irq))]
pub fn write<F: ::core::ops::FnOnce(&mut ::logging::LoggingFormatter)->::core::fmt::Result>(lvl: ::logging::Level, modname: &'static str, fcn: F) {
	let _ = fcn(&mut ::logging::getstream(lvl, modname));
}

#[doc(hidden)]
#[macro_export]
macro_rules! log{ ($lvl:expr, $modname:expr, $($arg:tt)*) => (
	if $crate::logging::enabled($lvl, $modname)
	{
		// NOTE: Keeps the logging out of the main path by using a closure
		$crate::logmacros::write($lvl, $modname, |s| { use core::fmt::Write; write!(s, $($arg)*) });
	}
	)}
/// Log a panic-level message (kernel intents to halt immediately after printing)
#[macro_export]
macro_rules! log_panic{   ($($arg:tt)*) => (log!($crate::logging::Level::LevelPanic,   module_path!(), $($arg)*))} 
/// "Error" - The current subsystem errored, and most likely will no longer function
#[macro_export]
macro_rules! log_error{   ($($arg:tt)*) => (log!($crate::logging::Level::LevelError,   module_path!(), $($arg)*))} 
/// Warning - Something unexpected happened, but it was recovered
#[macro_export]
macro_rules! log_warning{ ($($arg:tt)*) => (log!($crate::logging::Level::LevelWarning, module_path!(), $($arg)*))} 
/// Notice - Out of the ordinary, but not unexpected
#[macro_export]
macro_rules! log_notice{  ($($arg:tt)*) => (log!($crate::logging::Level::LevelNotice,  module_path!(), $($arg)*))} 
/// Information - Needs to be logged, but nothing to worry about
#[macro_export]
macro_rules! log_info{	($($arg:tt)*) => (log!($crate::logging::Level::LevelInfo,	module_path!(), $($arg)*))} 
/// Log - High-level debugging information
#[macro_export]
macro_rules! log_log{	 ($($arg:tt)*) => (log!($crate::logging::Level::LevelLog,	 module_path!(), $($arg)*))} 
/// Debug - Low level debugging information (values mostly)
#[macro_export]
macro_rules! log_debug{   ($($arg:tt)*) => (log!($crate::logging::Level::LevelDebug,   module_path!(), $($arg)*))} 
/// Trace calls to a function
#[macro_export]
macro_rules! log_function{($($arg:tt)*) => (log!($crate::logging::Level::LevelTrace, module_path!(), $($arg)*))} 
/// Trace - Very low level debugging information (action-by-action updates)
#[macro_export]
macro_rules! log_trace{
	($fmt:expr, $($arg:expr),*) => (log!($crate::logging::Level::LevelTrace, module_path!(), concat!("L{}: ",$fmt), line!() $(, $arg)*) );
	($str:expr) => (log_trace!($str, ));
	}

// vim: ft=rust

