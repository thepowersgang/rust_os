
#[macro_export]
macro_rules! log{ ($lvl:expr, $modname:expr, $($arg:tt)*) => (
	if $crate::logging::enabled($lvl, $modname)
	{
		use core::fmt::Writer;
		
		let _ = write!(&mut $crate::logging::getstream($lvl, $modname), $($arg)*);
	}
	)}
#[macro_export]
macro_rules! log_panic{   ($($arg:tt)*) => (log!($crate::logging::Level::LevelPanic,   module_path!(), $($arg)*))} 
#[macro_export]
macro_rules! log_error{   ($($arg:tt)*) => (log!($crate::logging::Level::LevelError,   module_path!(), $($arg)*))} 
#[macro_export]
macro_rules! log_warning{ ($($arg:tt)*) => (log!($crate::logging::Level::LevelWarning, module_path!(), $($arg)*))} 
#[macro_export]
macro_rules! log_notice{  ($($arg:tt)*) => (log!($crate::logging::Level::LevelNotice,  module_path!(), $($arg)*))} 
#[macro_export]
macro_rules! log_info{    ($($arg:tt)*) => (log!($crate::logging::Level::LevelInfo,    module_path!(), $($arg)*))} 
#[macro_export]
macro_rules! log_log{     ($($arg:tt)*) => (log!($crate::logging::Level::LevelLog,     module_path!(), $($arg)*))} 
#[macro_export]
macro_rules! log_debug{   ($($arg:tt)*) => (log!($crate::logging::Level::LevelDebug,   module_path!(), $($arg)*))} 
#[macro_export]
macro_rules! log_trace{
	($fmt:expr, $($arg:expr),*) => (log!($crate::logging::Level::LevelTrace, module_path!(), concat!("L{}: ",$fmt), line!() $(, $arg)*) );
	($str:expr) => (log_trace!($str, ));
	}

// vim: ft=rust

