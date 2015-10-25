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

impl<'a> Surface<'a>
{
	pub fn new(window: &Window, pos: Rect) -> Surface {
		const FILL_COLOUR: u32 = 0x33_00_00;
		let win_dims = window.get_dims();
		let max_dims = Dims { w: win_dims.w - pos.p.x, h: win_dims.h - pos.p.y };
		
		let pos = Rect {
			p: pos.p,
			d: Dims {
				w: ::std::cmp::min(pos.d.w, max_dims.w),
				h: ::std::cmp::min(pos.d.h, max_dims.h),
				},
			};
		Surface {
			window: window,
			cur_row: 0,
			row_buf: vec![FILL_COLOUR; (pos.d.w*C_CELL_DIMS.h) as usize],
			pos: pos,
			fill_colour: FILL_COLOUR,
		}
	}

	pub fn max_rows(&self) -> usize { (self.pos.d.h / C_CELL_DIMS.h) as usize }
	pub fn max_cols(&self) -> usize { (self.pos.d.w / C_CELL_DIMS.w) as usize }
	

	pub fn flush(&mut self) {
		self.window.blit_rect( self.pos.p.x, self.pos.p.y + self.cur_row as u32 * C_CELL_DIMS.h, self.pos.d.w, C_CELL_DIMS.h, &self.row_buf, self.pos.d.w as usize );
	}
	pub fn set_row(&mut self, row: usize) {
		self.flush();
		self.cur_row = row;
		for v in self.row_buf.iter_mut() { *v = self.fill_colour; }
	}

	/// Shift line's data leftwards (overwrites cell at `pos`, clearing the final cell)
	pub fn shift_line_left(&mut self, pos: usize) {
		let cw = C_CELL_DIMS.w as usize;
		let px_pos = pos * cw;
		let pen_cell = self.pos.d.w as usize - cw;
		let fc = self.fill_colour;
		for l in self.row_scanlines() {
			for i in (px_pos .. pen_cell) {
				l[i] = l[i+cw];
			}
			for v in l[pen_cell ..].iter_mut() {
				*v = fc;
			}
		}
	}
	/// Shift line's data rightwards (clearning cell at `pos`)
	pub fn shift_line_right(&mut self, pos: usize) {
		let cw = C_CELL_DIMS.w as usize;
		let px_pos = pos * cw;
		let pen_cell = self.pos.d.w as usize - cw;
		let fc = self.fill_colour;
		for l in self.row_scanlines() {
			for i in (px_pos .. pen_cell).rev() {
				l[i+cw] = l[i];
			}
			if (pos+1)*cw <= l.len() {
				for v in l[pos*cw .. (pos+1)*cw].iter_mut() {
					*v = fc;
				}
			}
		}
	}
	
	pub fn draw_cursor(&mut self, col: usize) {
		assert!(col < self.max_cols());
		let px_pos = col * C_CELL_DIMS.w as usize;
		for l in self.row_scanlines() {
			l[px_pos] = !l[px_pos];
		}
	}
	pub fn clear_cursor(&mut self, col: usize) {
		self.draw_cursor(col);
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
		let idx = unicode_to_cp437(cp);
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
