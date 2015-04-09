// Module: ::logging
//
//
use core::fmt::Write;
use core::result::Result::Ok;
use core::slice::{SliceExt};
use core::iter::Iterator;

/// Log level, ranging from a kernel panic down to tracing
#[derive(PartialEq,PartialOrd,Copy,Clone)]
pub enum Level
{
	/// Everything broke
	LevelPanic,
	/// Something broke
	LevelError,
	/// Recoverable
	LevelWarning,
 	/// Odd
	LevelNotice,
   	/// Interesting (least important for the user)
	LevelInfo,
    	/// General (highest developer-only level)
	LevelLog,
   	/// What
	LevelDebug,
  	/// Where
	LevelTrace,
}

pub enum Colour
{
	Default,
	Red,
	Yellow,
	Green,
	Blue,
	Purple,
}

#[doc(hidden)]
pub struct LoggingFormatter
{
	_lock_handle: ::arch::sync::HeldSpinlock<'static,()>,
	// NOTE: Must be second, forcing interrupts to be reenabled after the lock is released
	_irq_handle: ::arch::sync::HeldInterrupts,
}

/// Wrapper around a &-ptr that prints a hexdump of the passed data.
pub struct HexDump<'a,T:'a>(pub &'a T);

#[allow(non_upper_case_globals)]
static s_logging_lock: ::arch::sync::Spinlock<()> = spinlock_init!( () );

impl ::core::fmt::Display for Level
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
	/// Create a new logging formatter
	pub fn new() -> LoggingFormatter
	{
		LoggingFormatter {
			_irq_handle: ::arch::sync::hold_interrupts(),
			_lock_handle: s_logging_lock.lock()
		}
	}
	
	pub fn set_colour(&self, colour: Colour) {
		match colour
		{
		Colour::Default => ::arch::puts("\x1b[0m"),
		Colour::Red     => ::arch::puts("\x1b[31m"),
		Colour::Green   => ::arch::puts("\x1b[32m"),
		Colour::Yellow  => ::arch::puts("\x1b[33m"),
		Colour::Blue    => ::arch::puts("\x1b[34m"),
		Colour::Purple  => ::arch::puts("\x1b[35m"),
		}
	}
}

impl ::core::fmt::Write for LoggingFormatter
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
		::arch::puts("\x1b[0m\n");
	}
}

impl<'a,T:'a> HexDump<'a,T>
{
	/// Return the wrapped type as a &[u8]
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

impl<'a,T:'a> ::core::fmt::Debug for HexDump<'a,T>
{
	fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result
	{
		for (idx,v) in self.byteslice().iter().enumerate()
		{
			try!(write!(f, "{:02x} ", *v));
			if idx % 16 == 15 {
				try!(write!(f, "| "));
			}
		}
		Ok( () )
	}
}

//pub fn hex_dump_block(label: &str, data: &[u8])
//{
//	let mut pos = 0;
//	while pos + 16 <= data.len()
//	{
//		log_debug!("{} {:p}: {:?}  {:?}", label, &data[pos], HexDump(&data[pos .. pos+8]), HexDump(&data[pos+8 .. pos+16]));
//	}
//	if pos == data.len()
//	{
//	}
//	else if pos + 8 >= data.len()
//	{
//		//log_debug!("{} {:p}: {:?}", label, &data[pos], HexDump(&data[pos ..]));
//	}
//	else
//	{
//		//log_debug!("{} {:p}: {:?}  {:?}", label, &data[pos], HexDump(&data[pos .. pos+8]), HexDump(&data[pos+8 ..]));
//	}
//}

#[doc(hidden)]
/// Returns true if the passed combination of module and level is enabled
pub fn enabled(level: Level, modname: &str) -> bool
{
	match modname
	{
	"kernel::memory::heap" => (level < Level::LevelDebug),	// Heap only prints higher than debug
	_ => true,
	}
}

#[doc(hidden)]
/// Returns a logging formatter
pub fn getstream(level: Level, modname: &str) -> LoggingFormatter
{
	assert!( enabled(level, modname) );
	let mut rv = LoggingFormatter::new();
	rv.set_colour(match level {
		Level::LevelPanic   => Colour::Purple,
		Level::LevelError   => Colour::Red,
		Level::LevelWarning => Colour::Yellow,
		Level::LevelNotice  => Colour::Green,
		_ => Colour::Default,
		});
	let _ = write!(&mut rv, "{:8}{} [{:6}] - ", ::time::ticks(), level, modname);
	rv
}


// vim: ft=rust

