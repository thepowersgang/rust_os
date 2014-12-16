// Module: ::logging
//
//
use core::fmt::FormatWriter;
use core::option::Option::{mod,Some,None};
use core::result::Result::{Ok,Err};
use core::slice::{SlicePrelude};

#[deriving(PartialEq,PartialOrd,Copy)]
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
	lock_handle: ::arch::sync::HeldSpinlock<'static,()>,
}

pub struct HexDump<'a,T:'a>(pub &'a T);

// NOTE: Has to be a spinlock, stops interrupts while held
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
			lock_handle: s_logging_lock.lock()
		}
	}
}

impl ::core::fmt::FormatWriter for LoggingFormatter
{
	fn write(&mut self, bytes: &[u8]) -> ::core::fmt::Result
	{
		match ::core::str::from_utf8(bytes)
		{
		Some(s) => ::arch::puts(s),
		None => {
			let rs = unsafe { ::core::mem::transmute::<_,::core::raw::Slice<u8>>(bytes) };
			panic!("LoggingFormatter.write bytes={}+{}", rs.data, rs.len);
			}
		}
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

