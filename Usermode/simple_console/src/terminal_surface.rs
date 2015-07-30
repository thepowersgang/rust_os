// Tifflin OS - simple_console
// - By John Hodge (thePowersGang)
//
// Simplistic console, used as a quick test case (fullscreen window)

// Bitmap font used by this module is in another file
include!("../../../Graphics/font_cp437_8x16.rs");

use syscalls::gui::{Window, Dims, Pos, Rect, Colour};

const C_CELL_DIMS: Dims = Dims { w: 8, h: 16 };

pub struct Surface<'a>
{
	window: &'a Window,
	pos: Rect,
	cur_row: usize,
	row_buf: Vec<u32>,
	fill_colour: u32,
}

/// Trait to provde 'is_combining', used by render code
trait UnicodeCombining
{
	fn is_combining(&self) -> bool;
}

impl<'a> Surface<'a>
{
	pub fn new(window: &Window, pos: Rect) -> Surface {
		Surface {
			window: window,
			cur_row: 0,
			row_buf: ::std::iter::repeat(0).take( (pos.d.w*C_CELL_DIMS.h) as usize ).collect(),
			pos: pos,
			fill_colour: 0x33_00_00,
		}
	}
	

	pub fn flush(&mut self) {
		self.window.blit_rect( self.pos.p.x, self.pos.p.y + self.cur_row as u32 * C_CELL_DIMS.h, self.pos.d.w, C_CELL_DIMS.h, &self.row_buf );
	}
	pub fn set_row(&mut self, row: usize) {
		self.flush();
		self.cur_row = row;
	}
	
	/// Writes a single codepoint to the display
	///
	/// Returns true if the character caused a cell change (i.e. it wasn't a combining character)
	pub fn putc(&mut self, col: usize, colour: Colour, c: char) -> bool
	{
		// If the character was a combining AND it's not at the start of a line,
		// render atop the previous cell
		if c.is_combining() && col > 0 {
			self.render_char(col-1, colour, c);
			false
		}
		// Otherwise, wipe the cell and render into it
		else {
			self.clear_cell(col);
			self.render_char(col, colour, c);
			true
		}
	}
	
	fn row_scanlines(&mut self) -> ::std::slice::ChunksMut<u32> {
		self.row_buf.chunks_mut(self.pos.d.w as usize)
	}
	
	// Low-level rendering
	/// Clear a character cell
	fn clear_cell(&mut self, col: usize)
	{
		let ofs = col * C_CELL_DIMS.w as usize;
		let fill = self.fill_colour;
		for r in self.row_scanlines() {
			for v in r[ofs .. ofs + C_CELL_DIMS.w as usize].iter_mut() {
				*v = fill;
			}
		}
	}
	/// Actually does the rendering
	fn render_char(&mut self, col: usize, colour: Colour, cp: char)
	{
		let idx = match cp as u32
			{
			32 ... 0x7E => cp as u8,
			_ => b'?',
			};
		//log_trace!("KernelLog::render_char({:?}, {:?}, '{}') idx={}", pos, colour, cp, idx);
		
		let bitmap = &S_FONTDATA[idx as usize];
		
		// Actual render!
		let bx: usize = C_CELL_DIMS.w as usize * col;
		for row in (0 .. 16)
		{
			let byte = &bitmap[row as usize];
			//let r = self.buffer_handle.scanline_rgn_mut(by as usize + row, bx as usize, 8); 
			let base: usize = row * self.pos.d.w as usize + bx;
			let r = &mut self.row_buf[base .. base + C_CELL_DIMS.w as usize]; 
			for col in (0usize .. 8)
			{
				if (byte >> 7-col) & 1 != 0 {
					r[col] = colour.as_argb32();
				}
			}
		}
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
