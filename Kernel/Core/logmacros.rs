
#![macro_escape]	// Let macros be accessible by parent

macro_rules! log{ ($lvl:expr, $modname:expr, $($arg:tt)*) => (
	if ::logging::enabled($lvl, $modname)
	{
		use core::fmt::FormatWriter;
		let _ = write!(&mut ::logging::getstream($lvl, $modname), $($arg)*);
	}
	)}
macro_rules! log_panic{   ($($arg:tt)*) => (log!(::logging::Level::LevelPanic,   module_path!(), $($arg)*))} 
macro_rules! log_error{   ($($arg:tt)*) => (log!(::logging::Level::LevelError,   module_path!(), $($arg)*))} 
macro_rules! log_warning{ ($($arg:tt)*) => (log!(::logging::Level::LevelWarning, module_path!(), $($arg)*))} 
macro_rules! log_notice{  ($($arg:tt)*) => (log!(::logging::Level::LevelNotice,  module_path!(), $($arg)*))} 
macro_rules! log_info{    ($($arg:tt)*) => (log!(::logging::Level::LevelInfo,    module_path!(), $($arg)*))} 
macro_rules! log_log{     ($($arg:tt)*) => (log!(::logging::Level::LevelLog,     module_path!(), $($arg)*))} 
macro_rules! log_debug{   ($($arg:tt)*) => (log!(::logging::Level::LevelDebug,   module_path!(), $($arg)*))} 
macro_rules! log_trace{ ($fmt:expr $(, $arg:expr)*) => (
	log!(::logging::Level::LevelTrace, module_path!(), concat!("L{}: ",$fmt), line!() $(, $arg)*)
	) }

// vim: ft=rust

