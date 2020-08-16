// "Tifflin" Kernel
// - By John Hodge (thePowersGang)
//
// Core/gui/kernel_log.rs
/// Kernel log output (and debug terminal)
//
// Manages a set of windows giving a view into the kernel
// - Kernel log (current) : Contains the most recent kernel log messages
// - Logo
// - TODO: Kernel log (history) : A searchable/filterable/scrollable kernel log
// - TODO: Console 
#[allow(unused_imports)]
use kernel::prelude::*;

use super::windows::{WindowGroupHandle,WindowHandle};
use super::{Colour,Dims,Pos,Rect};
use core::fmt;
use kernel::sync::mutex::{LazyMutex,HeldLazyMutex};

// Bitmap font used by this module is in another file
include!("../../../Graphics/font_cp437_8x16.rs");

// Raw bitmap logo (already encoded with dims and as a rust array)
include!("../../../Graphics/.output/shared/logo.rs");

struct KernelLog
{
	_wgh: WindowGroupHandle,
	wh: WindowHandle,
	_logo_wh: WindowHandle,
	cur_line: u32,
	
	buffer_handle: super::windows::BufHandle,
}

/// Character position
#[derive(Copy,Clone,Debug)]
struct CharPos(u32,u32);

struct LogWriter
{
	log: HeldLazyMutex<'static,KernelLog>,
	pos: CharPos,
	colour: Colour,
}

/// Trait to provde 'is_combining', used by render code
trait UnicodeCombining
{
	fn is_combining(&self) -> bool;
}

const C_CELL_DIMS: Dims = Dims { w: 8, h: 16 };
static S_KERNEL_LOG: LazyMutex<KernelLog> = lazymutex_init!();

#[doc(hidden)]
pub fn init()
{
	// Create window (and structure)
	S_KERNEL_LOG.init(|| KernelLog::new());
	
	//super::register_dims_update(|| S_KERNEL_LOG.lock().update_dims());
	//S_KERNEL_LOG.lock().register_input();

	{
		use core::fmt::Write;
		write!(&mut LogWriter::new(Colour::def_green() ), "{}", ::kernel::build_info::version_string()).unwrap();
		write!(&mut LogWriter::new(Colour::def_yellow()), "> {}", ::kernel::build_info::build_string()).unwrap();
	}
	
	// Populate kernel logging window with accumulated logs
	// TODO: 
	// Register to recieve logs
}

impl KernelLog
{
	fn new() -> KernelLog
	{
		// TODO: Register to somehow be informed when dimensions change
		// - Is this particular call bad for bypassing the GUI? Or is this acceptable
		let max_dims = match ::kernel::metadevs::video::get_display_for_pos( Pos::new(0,0) )
			{
			Ok(display) => display.dims(),
			Err(_) => {
				log_warning!("No display at (0,0)");
				Dims::new(0,0)
				},
			};
	
		// Kernel's window group	
		let mut wgh = WindowGroupHandle::alloc("Kernel");
		
		// - Log Window
		let mut wh = wgh.create_window("Kernel Log");
		//wh.set_pos(Pos::new(0,0));
		//wh.resize(max_dims);
		wh.maximise();
		let log_buf_handle = wh.get_buffer();
		
		// - Fancy logo window
		let dims = Dims::new(S_LOGO_DIMS.0,S_LOGO_DIMS.1);
		let mut logo_wh = wgh.create_window("Logo");
		if max_dims != Dims::new(0,0)
		{
			logo_wh.set_pos(Pos::new(max_dims.w-dims.w, 0));
			logo_wh.resize(dims);
			logo_wh.blit_rect( Rect::new_pd(Pos::new(0,0),dims), &S_LOGO_DATA, dims.w as usize );
		}
			
		if max_dims != Dims::new(0,0)
		{
			// > Show windows in reverse render order
			wh.show();
			logo_wh.show();
		}
		
		// Return struct populated with above handles	
		KernelLog {
			_wgh: wgh,
			wh: wh,
			_logo_wh: logo_wh,
			cur_line: 0,
			buffer_handle: log_buf_handle,
		}
	}
	
	/// Scroll the display up a step, revealing a new line
	fn scroll_up(&mut self)
	{
		self.cur_line += 1;
	}
	
	/// Write a string to the log display (at the given character position)
	fn write_text(&self, mut pos: CharPos, colour: Colour, text: &str) -> CharPos
	{
		if self.buffer_handle.dims().w == 0 || self.buffer_handle.dims().h == 0 {
			return pos;
		}
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
	//#[req_safe(taskswitch)]	//< Must be safe to call from within a spinlock
	fn flush(&self)
	{
		// Poke the WM and tell it to reblit us
		self.wh.redraw();
	}
	
	/// Writes a single codepoint to the display
	///
	/// Returns true if the character caused a cell change (i.e. it wasn't a combining character)
	fn putc(&self, pos: CharPos, colour: Colour, c: char) -> bool
	{
		// If the character was a combining AND it's not at the start of a line,
		// render atop the previous cell
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
	fn clear_cell(&self, pos: CharPos)
	{
		let Pos { x: bx, y: by } = pos.to_pixels();
		for row in 0 .. 16
		{
			let r = self.buffer_handle.scanline_rgn_mut(by as usize + row, bx as usize, 8); 
			for col in 0 .. 8
			{
				r[col] = 0;
			}
		}
	}
	/// Actually does the rendering
	//#[req_safe(taskswitch)]	//< Must be safe to call from within a spinlock
	fn render_char(&self, pos: CharPos, colour: Colour, cp: char)
	{
		if self.buffer_handle.dims().width() == 0 {
			return ;
		}

		let idx = unicode_to_cp437(cp);
		//log_trace!("KernelLog::render_char({:?}, {:?}, '{}') idx={}", pos, colour, cp, idx);
		
		let bitmap = &S_FONTDATA[idx as usize];
		
		// Actual render!
		let Pos { x: bx, y: by } = pos.to_pixels();
		for row in 0 .. 16
		{
			let byte = &bitmap[row as usize];
			let r = self.buffer_handle.scanline_rgn_mut(by as usize + row, bx as usize, 8); 
			for col in 0 .. 8
			{
				if (byte >> 7-col) & 1 != 0 {
					r[col] = colour.as_argb32();
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
		Pos::new( (self.1 * C_CELL_DIMS.w) as u32, (self.0 * C_CELL_DIMS.h) as u32 )
	}
}

impl LogWriter
{
	pub fn new(colour: Colour) -> LogWriter
	{
		let mut log = S_KERNEL_LOG.lock();
		log.scroll_up();
		LogWriter {
			pos: CharPos(log.cur_line-1,0),
			colour: colour,
			log: log,
		}
	}
}
impl fmt::Write for LogWriter
{
	fn write_str(&mut self, s: &str) -> fmt::Result
	{
		self.pos = self.log.write_text(self.pos, self.colour, s);
		Ok( () )
	}
}
impl ::core::ops::Drop for LogWriter
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
		0x0300 ..= 0x036F => true,
		0x1AB0 ..= 0x1AFF => true,
		0x1DC0 ..= 0x1DFF => true,
		0x20D0 ..= 0x20FF => true,
		0xFE20 ..= 0xFE2F => true,
		_ => false,
		}
	}
}

