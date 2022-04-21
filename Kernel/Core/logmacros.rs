
#[inline(never)]
#[doc(hidden)]
//#[cfg_attr(not(test_shim),req_safe(irq))]
pub fn write<F: ::core::ops::FnOnce(&mut crate::logging::LoggingFormatter)->::core::fmt::Result>(lvl: crate::logging::Level, modname: &'static str, fcn: F) {
	let _ = fcn(&mut crate::logging::getstream(lvl, modname));
}

#[doc(hidden)]
#[macro_export]
macro_rules! log{ ($lvl:expr, $modname:expr, $($arg:tt)*) => (
	if $crate::logging::enabled($lvl, $modname)
	{
		// NOTE: Keeps the logging out of the main path by using a closure
		match format_args!($($arg)*)
		{
		args => $crate::logmacros::write($lvl, $modname, |s| { use core::fmt::Write; s.write_fmt(args) }),
		}
	}
	)}
/// Log a panic-level message (kernel intents to halt immediately after printing)
#[macro_export]
macro_rules! log_panic{   ($($arg:tt)*) => (log!($crate::logging::Level::Panic,   module_path!(), $($arg)*))} 
/// "Error" - The current subsystem errored, and most likely will no longer function
#[macro_export]
macro_rules! log_error{   ($($arg:tt)*) => (log!($crate::logging::Level::Error,   module_path!(), $($arg)*))} 
/// Warning - Something unexpected happened, but it was recovered
#[macro_export]
macro_rules! log_warning{ ($($arg:tt)*) => (log!($crate::logging::Level::Warning, module_path!(), $($arg)*))} 
/// Notice - Out of the ordinary, but not unexpected
#[macro_export]
macro_rules! log_notice{  ($($arg:tt)*) => (log!($crate::logging::Level::Notice,  module_path!(), $($arg)*))} 
/// Information - Needs to be logged, but nothing to worry about
#[macro_export]
macro_rules! log_info{	($($arg:tt)*) => (log!($crate::logging::Level::Info,	module_path!(), $($arg)*))} 
/// Log - High-level debugging information
#[macro_export]
macro_rules! log_log{	 ($($arg:tt)*) => (log!($crate::logging::Level::Log,	 module_path!(), $($arg)*))} 
/// Debug - Low level debugging information (values mostly)
#[macro_export]
macro_rules! log_debug{   ($($arg:tt)*) => (log!($crate::logging::Level::Debug,   module_path!(), $($arg)*))} 
/// Trace calls to a function
#[macro_export]
macro_rules! log_function{($($arg:tt)*) => (log!($crate::logging::Level::Trace, module_path!(), $($arg)*))} 
/// Trace - Very low level debugging information (action-by-action updates)
#[macro_export]
macro_rules! log_trace{
	($fmt:expr, $($arg:expr),*) => (log!($crate::logging::Level::Trace, module_path!(), concat!("L{}: ",$fmt), line!() $(, $arg)*) );
	($str:expr) => (log_trace!($str, ));
	}

// vim: ft=rust

