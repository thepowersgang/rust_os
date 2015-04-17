// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// arch/amd64/boot.rs
///! Kernel logging framework
///!
///! All kernel logging goes through this module, using the `log_*` macros, each corresponding
///! to a one of the logging levels in `Levels`.
// TODO: Support registerable log sinks, where all >Debug logs go (why not all?)
// > Cache/buffer sink
// > Display sink
// > Serial sink
use _common::*;
use core::fmt::{self,Write};
use arch::sync::Spinlock;

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

#[doc(hidden)]
pub enum Colour
{
	Default,
	Red,
	Yellow,
	Green,
	Blue,
	Purple,
	Grey,
}

#[doc(hidden)]
pub struct LoggingFormatter<'a>
{
	_lock_handle: ::arch::sync::HeldSpinlock<'a,()>,
	
	//_lock_handle: ::arch::sync::HeldSpinlock<'a,Sinks>,
	
	// NOTE: Must be second, forcing interrupts to be reenabled after the lock is released
	_irq_handle: ::arch::sync::HeldInterrupts,
}

/// Wrapper around a &-ptr that prints a hexdump of the passed data.
pub struct HexDump<'a,T:'a>(pub &'a T);

#[allow(non_upper_case_globals)]
static s_logging_lock: ::arch::sync::Spinlock<()> = spinlock_init!( () );

trait Sink
{
	fn start(&mut self, level: Level, timestamp: i64, source: &'static str);
	fn write(&mut self, data: &str);
	fn end(&mut self);
}
struct Sinks
{
	serial: serial::Sink,
	memory: Option<memory::Sink>,
	video: Option<video::Sink>,
}
static S_LOGGING_SINKS: Spinlock<Sinks> = spinlock_init!( Sinks { serial: serial::Sink, memory: None, video: None } );

mod serial
{
	use _common::*;
	use super::{Level,Colour};
	use core::{fmt,ops};
	use core::marker::PhantomData;
	
	pub struct Sink;
	impl super::Sink for Sink
	{
		fn start(&mut self, level: Level, timestamp: i64, source: &'static str) {
			use core::fmt::Write;
			self.set_colour(level.to_colour());
			write!(self, "{:8}{} [{:6}] - ", timestamp, level, source).unwrap();
		}
		fn write(&mut self, s: &str) {
			::arch::puts(s);
		}
		fn end(&mut self) {
			::arch::puts("\x1b[0m\n");
		}
	}
	impl fmt::Write for Sink
	{
		fn write_str(&mut self, s: &str) -> fmt::Result {
			::arch::puts(s);
			Ok( () )
		}
	}
	impl Sink
	{
		/// Set the output colour of the formatter
		pub fn set_colour(&self, colour: Colour) {
			match colour
			{
			Colour::Default => ::arch::puts("\x1b[0m"),
			Colour::Red     => ::arch::puts("\x1b[31m"),
			Colour::Green   => ::arch::puts("\x1b[32m"),
			Colour::Yellow  => ::arch::puts("\x1b[33m"),
			Colour::Blue    => ::arch::puts("\x1b[34m"),
			Colour::Purple  => ::arch::puts("\x1b[35m"),
			Colour::Grey    => ::arch::puts("\x1b[1;30m"),
			}
		}
	}
}

mod memory
{
	use _common::*;
	use super::Level;
	use core::{fmt,ops};
	
	pub struct Sink
	{
		lines: ::lib::ring_buffer::RingBuf<LogMessage>,
	}
	struct LogMessage
	{
		time: i64,
		level: Level,
		source: &'static str,
		data: String,
	}
	impl super::Sink for Sink
	{
		fn start(&mut self, level: Level, timestamp: i64, source: &'static str) {
			todo!("MemorySink");
		}
		fn write(&mut self, s: &str) {
			self.lines.back_mut().unwrap().data.push_str(s);
		}
		fn end(&mut self) {
			unimplemented!();
		}
	}
}

mod video
{
	use _common::*;
	use super::Level;
	use core::{fmt,ops};
	use core::marker::PhantomData;
	
	pub struct Sink;
	
	pub struct Writer<'a>(PhantomData<&'a mut Sink>);
	
	impl super::Sink for Sink
	{
		fn start(&mut self, level: Level, timestamp: i64, source: &'static str) {
			todo!("VideoSink");
		}
		fn write(&mut self, s: &str) {
			unimplemented!();
		}
		fn end(&mut self) {
			unimplemented!();
		}
	}
}

impl Level
{
	fn to_flag(&self) -> char
	{
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
	}
	fn to_colour(&self) -> Colour
	{
		match *self
		{
		Level::LevelPanic   => Colour::Purple,
		Level::LevelError   => Colour::Red,
		Level::LevelWarning => Colour::Yellow,
		Level::LevelNotice  => Colour::Green,
		Level::LevelLog     => Colour::Blue,
		Level::LevelTrace   => Colour::Grey,
		_ => Colour::Default,
		}
	}
}

impl fmt::Display for Level
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.to_flag())
	}
}

impl<'a> LoggingFormatter<'a>
{
	/// Create a new logging formatter
	pub fn new(_level: Level, _modname: &str) -> LoggingFormatter<'static>
	{
		let rv = LoggingFormatter {
				_irq_handle: ::arch::sync::hold_interrupts(),
				_lock_handle: s_logging_lock.lock()
			};
		//let ts = ::time::ticks();
		//rv.lock_handle.serial.start(ts, level, modname);
		//rv.lock_handle.memory.map(|x| x.start(ts, level, modname) );
		//rv.lock_handle.video.map(|x| x.start(ts, level, modname) );
		rv
	}
	
	/// Set the output colour of the formatter
	pub fn set_colour(&self, colour: Colour) {
		match colour
		{
		Colour::Default => ::arch::puts("\x1b[0m"),
		Colour::Red     => ::arch::puts("\x1b[31m"),
		Colour::Green   => ::arch::puts("\x1b[32m"),
		Colour::Yellow  => ::arch::puts("\x1b[33m"),
		Colour::Blue    => ::arch::puts("\x1b[34m"),
		Colour::Purple  => ::arch::puts("\x1b[35m"),
		Colour::Grey    => ::arch::puts("\x1b[1;30m"),
		}
	}
}

impl<'a> fmt::Write for LoggingFormatter<'a>
{
	fn write_str(&mut self, s: &str) -> fmt::Result
	{
		::arch::puts(s);
		Ok( () )
	}
}
impl<'a> ::core::ops::Drop for LoggingFormatter<'a>
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

impl<'a,T:'a> fmt::Debug for HexDump<'a,T>
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
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
	"kernel::memory::phys" => (level < Level::LevelTrace),	// PMM only prints >Trace
	_ => true,
	}
}

#[doc(hidden)]
/// Returns a logging formatter
pub fn getstream(level: Level, modname: &str) -> LoggingFormatter
{
	assert!( enabled(level, modname) );
	let mut rv = LoggingFormatter::new(level, modname);
	rv.set_colour(level.to_colour());
	let _ = write!(&mut rv, "{:8}{} [{:6}] - ", ::time::ticks(), level, modname);
	rv
}


// vim: ft=rust

