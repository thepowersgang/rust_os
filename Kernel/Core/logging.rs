// Module: ::logging
//
//
use core::fmt::Writer;
use core::result::Result::Ok;
use core::slice::{SliceExt};

#[derive(PartialEq,PartialOrd,Copy)]
pub enum Level
{
	LevelPanic,  	// Everything broke
	LevelError,  	// Something broke
	LevelWarning,	// Recoverable
	LevelNotice, 	// Odd
	LevelInfo,   	// Interesting
	LevelLog,    	// General
	LevelDebug,   	// What
	LevelTrace,  	// Where
}

struct LoggingFormatter
{
	_lock_handle: ::arch::sync::HeldSpinlock<'static,()>,
}

pub struct HexDump<'a,T:'a>(pub &'a T);

// NOTE: Has to be a spinlock, stops interrupts while held
#[allow(non_upper_case_globals)]
static s_logging_lock: ::arch::sync::Spinlock<()> = spinlock_init!( () );

impl ::core::fmt::Show for Level
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
		write!(f, "{}",
			match *self
			{
			Level::LevelPanic   => 'k',
			Level::LevelError   => 'e',
			Level::LevelWarning => 'w',
			Level::LevelNotice  => 'n',
			Level::LevelInfo    => 'i',
			Level::LevelLog     => 'l',
			Level::LevelDebug   => 'd',
			Level::LevelTrace   => 't',
			}
			)
	}
}

impl LoggingFormatter
{
	pub fn new() -> LoggingFormatter
	{
		LoggingFormatter {
			_lock_handle: s_logging_lock.lock()
		}
	}
}

impl ::core::fmt::Writer for LoggingFormatter
{
	fn write_str(&mut self, s: &str) -> ::core::fmt::Result
	{
		::arch::puts(s);
		Ok( () )
	}
}
impl ::core::ops::Drop for LoggingFormatter
{
	fn drop(&mut self)
	{
		::arch::puts("\n");
	}
}

impl<'a,T:'a> HexDump<'a,T>
{
	fn byteslice(&self) -> &[u8]
	{
		let size = ::core::mem::size_of::<T>();
		unsafe {
			::core::mem::transmute(::core::raw::Slice {
				data: self.0 as *const T as *const u8,
				len: size,
			})
		}
	}
}

impl<'a,T:'a> ::core::fmt::Show for HexDump<'a,T>
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		for i in self.byteslice().iter()
		{
			try!(write!(f, "{:02x} ", *i));
		}
		Ok( () )
	}
}

pub fn enabled(_level: Level, _modname: &str) -> bool
{
	//if _modname == "main::::memory::heap" && _level == Level::LevelDebug {
	//	return false;
	//}
	true
}

pub fn getstream(level: Level, modname: &str) -> LoggingFormatter
{
	assert!( enabled(level, modname) );
	let mut rv = LoggingFormatter::new();
	let _ = write!(&mut rv, "{:8}{} [{:6}] - ", ::time::ticks(), level, modname);
	rv
}


// vim: ft=rust

