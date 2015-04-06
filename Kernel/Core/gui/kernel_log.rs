// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/kernel_log.rs
// - Kernel log output (and debug)

use _common::*;

use super::windows::{WindowGroupHandle,WindowHandle};
use super::{Colour,Dims,Pos,Rect};
use core::fmt;

// Bitmap font used by this module is in another file
include!("font_cp437_8x16.rs");

struct KernelLog
{
	wgh: WindowGroupHandle,
	wh: WindowHandle,
	cur_line: u32,
}

/// Character position
#[derive(Copy,Clone,Debug)]
struct CharPos(u32,u32);

struct LogWriter<'a>
{
	log: &'a mut KernelLog,
	pos: CharPos,
	colour: Colour,
}

/// Trait to provde 'is_combining', used by render code
trait UnicodeCombining
{
	fn is_combining(&self) -> bool;
}

const C_CELL_DIMS: Dims = Dims(8,16);
static S_KERNEL_LOG: ::sync::mutex::LazyMutex<KernelLog> = lazymutex_init!();

pub fn init()
{
	// Create window (and structure)
	S_KERNEL_LOG.init(|| KernelLog::new());
	
	// DEBUG: Print something
	{
		use core::fmt::Write;
		let mut lh = S_KERNEL_LOG.lock();
		let mut w = LogWriter::new(&mut lh);
		write!(&mut w, "???l [] ");
		w.set_colour(Colour::def_yellow());
		write!(&mut w, "Hello World!");
	}
	
	// Populate kernel logging window with accumulated logs
	// Register to recieve logs
}

impl KernelLog
{
	fn new() -> KernelLog
	{
		let mut wgh = WindowGroupHandle::alloc("Kernel");
		let wh = wgh.create_window();
		KernelLog {
			wgh: wgh,
			wh: wh,
			cur_line: 0
		}
	}
	
	/// Scroll the display up a step, revealing a new line
	fn scroll_up(&mut self)
	{
		self.cur_line += 1;
	}
	
	/// Write a string to the log display (at the given character position)
	fn write_text(&mut self, mut pos: CharPos, colour: Colour, text: &str) -> CharPos
	{
		for c in text.chars()
		{
			if self.putc(pos, colour, c)
			{
				pos = pos.next();
			}
		}
		pos
	}
	/// Flush changes
	fn flush(&mut self)
	{
		// Poke the WM and tell it to reblit us
		self.wh.redraw();
	}
	
	/// Writes a single codepoint to the display
	///
	/// Returns true if the character caused a cell change (i.e. it wasn't a combining character)
	fn putc(&mut self, pos: CharPos, colour: Colour, c: char) -> bool
	{
		// If the character was a combining AND
		//  it's not at the start of a line, render atop the previous cell
		if c.is_combining() && pos.col() > 0 {
			self.render_char(pos.prev(), colour, c);
			false
		}
		// Otherwise, wipe the cell and render into it
		else {
			self.clear_cell(pos);
			self.render_char(pos, colour, c);
			true
		}
	}
	
	// Low-level rendering
	/// Clear a character cell
	fn clear_cell(&mut self, pos: CharPos)
	{
		self.wh.fill_rect( Rect(pos.to_pixels(),C_CELL_DIMS), Colour::def_black());
	}
	/// Actually does the rendering
	fn render_char(&mut self, pos: CharPos, colour: Colour, cp: char)
	{
		let idx = match cp as u32
			{
			32 ... 0x7E => cp as u8,
			_ => b'?',
			};
		
		let bitmap = &S_FONTDATA[idx as usize];
		
		// Actual render!
		let Pos(bx, by) = pos.to_pixels();
		for row in (0 .. 16)
		{
			let byte = &bitmap[row as usize];
			for col in (0 .. 8)
			{
				if byte & (1 << col as usize) != 0 {
					self.wh.pset(Pos(bx+col,by+row), colour);
				}
			}
		}
	}
}

impl CharPos
{
	fn col(&self) -> u32 { self.1 }
	fn next(self) -> CharPos { CharPos(self.0, self.1+1) }
	fn prev(self) -> CharPos { CharPos(self.0, self.1-1) }
	fn to_pixels(self) -> Pos {
		Pos( (self.0 * C_CELL_DIMS.0) as i32, (self.1 * C_CELL_DIMS.1) as i32 )
	}
}

impl<'a> LogWriter<'a>
{
	pub fn new(log: &mut KernelLog) -> LogWriter
	{
		log.scroll_up();
		LogWriter {
			pos: CharPos(log.cur_line,0),
			colour: Colour::def_white(),
			log: log,
		}
	}
	
	pub fn set_colour(&mut self, colour: Colour)
	{
		self.colour = colour;
	}
}
impl<'a> fmt::Write for LogWriter<'a>
{
	fn write_str(&mut self, s: &str) -> fmt::Result
	{
		self.pos = self.log.write_text(self.pos, self.colour, s);
		Ok( () )
	}
}
impl<'a> ::core::ops::Drop for LogWriter<'a>
{
	fn drop(&mut self)
	{
		self.log.flush();
	}
}

impl UnicodeCombining for char
{
	fn is_combining(&self) -> bool
	{
		match *self as u32
		{
		// Ranges from wikipedia:Combining_Character
		0x0300 ... 0x036F => true,
		0x1AB0 ... 0x1AFF => true,
		0x1DC0 ... 0x1DFF => true,
		0x20D0 ... 0x20FF => true,
		0xFE20 ... 0xFE2F => true,
		_ => false,
		}
	}
}

