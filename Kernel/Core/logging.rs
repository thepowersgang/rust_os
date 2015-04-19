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
	lock_handle: ::arch::sync::HeldSpinlock<'a,Sinks>,
	
	// NOTE: Must be second, forcing interrupts to be reenabled after the lock is released
	_irq_handle: ::arch::sync::HeldInterrupts,
}

/// Wrapper around a &-ptr that prints a hexdump of the passed data.
pub struct HexDump<'a,T:'a>(pub &'a T);

static S_LOGGING_LOCK: Spinlock<Sinks> = spinlock_init!( Sinks { serial: serial::Sink, memory: None, video: None } );

trait Sink
{
	fn start(&mut self, timestamp: ::time::TickCount, level: Level, source: &'static str);
	fn write(&mut self, data: &str);
	fn end(&mut self);
}
struct Sinks
{
	serial: serial::Sink,
	memory: Option<memory::Sink>,
	video: Option<video::Sink>,
}

mod serial
{
	use _common::*;
	use super::{Level,Colour};
	use core::{fmt,ops};
	use core::marker::PhantomData;
	
	pub struct Sink;
	impl super::Sink for Sink
	{
		fn start(&mut self, timestamp: ::time::TickCount, level: Level, source: &'static str) {
			use core::fmt::Write;
			self.set_colour(level.to_colour());
			write!(self, "{:6}{} [{}] - ", timestamp, level, source).unwrap();
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
	// Buffer for log data
	// Temp hack until type-level ints are avaliable
	struct LogDataBuf([u8;160]);
	impl LogDataBuf {
		fn new() -> LogDataBuf { LogDataBuf([0; 160]) }
	}
	impl ::core::convert::AsRef<[u8]> for LogDataBuf {
		fn as_ref(&self) -> &[u8] { &self.0 }
	}
	impl ::core::convert::AsMut<[u8]> for LogDataBuf {
		fn as_mut(&mut self) -> &mut [u8] { &mut self.0 }
	}
	struct LogMessage
	{
		time: ::time::TickCount,
		level: Level,
		source: &'static str,
		data: ::lib::string::FixedString<LogDataBuf>,
	}
	impl Sink
	{
		pub fn new() -> Sink {
			Sink {
				lines: ::lib::ring_buffer::RingBuf::new(256),	// 256 log of scrollback
			}
		}
	}
	impl super::Sink for Sink
	{
		fn start(&mut self, timestamp: ::time::TickCount, level: Level, source: &'static str) {
			let new_line = LogMessage {
				time: timestamp, level: level, source: source,
				data: ::lib::string::FixedString::new(LogDataBuf::new())
				};
			match self.lines.push_back( new_line )
			{
			Ok(_) => {},
			Err(new_line) => {
				todo!("Handle rollover of kernel log");
				},
			}
		}
		fn write(&mut self, s: &str) {
			self.lines.back_mut().unwrap().data.push_str(s);
		}
		fn end(&mut self) {
			// No action required
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
		fn start(&mut self, timestamp: ::time::TickCount, level: Level, source: &'static str) {
			// Acquire a writer from the GUI
			// - TODO: requires acquiring the lock on the kernel log, which is a Mutex, and may already be held.
			// Write header
			todo!("VideoSink");
		}
		fn write(&mut self, s: &str) {
			// Pass through
			unimplemented!();
		}
		fn end(&mut self) {
			// Drop writer (replace with None)
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

impl Sinks
{
	pub fn foreach_mut<Fcn: FnMut(&mut Sink)>(&mut self, mut f: Fcn)
	{
		f(&mut self.serial);
		self.memory.as_mut().map(|x| f(x));
		self.video .as_mut().map(|x| f(x));
	}
}

impl<'a> LoggingFormatter<'a>
{
	/// Create a new logging formatter
	pub fn new(level: Level, modname: &'static str) -> LoggingFormatter<'static>
	{
		let mut rv = LoggingFormatter {
				_irq_handle: ::arch::sync::hold_interrupts(),
				lock_handle: S_LOGGING_LOCK.lock()
			};
		let ts = ::time::ticks();
		rv.lock_handle.foreach_mut(|x| x.start(ts, level, modname));
		rv
	}
}

impl<'a> fmt::Write for LoggingFormatter<'a>
{
	fn write_str(&mut self, s: &str) -> fmt::Result
	{
		self.lock_handle.foreach_mut(|x| x.write(s));
		Ok( () )
	}
}
impl<'a> ::core::ops::Drop for LoggingFormatter<'a>
{
	fn drop(&mut self)
	{
		self.lock_handle.foreach_mut(|x| x.end());
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
pub fn getstream(level: Level, modname: &'static str) -> LoggingFormatter
{
	assert!( enabled(level, modname) );
	LoggingFormatter::new(level, modname)
}


// vim: ft=rust

